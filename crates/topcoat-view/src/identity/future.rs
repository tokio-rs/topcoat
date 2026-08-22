use std::{
    pin::Pin,
    task::{Context, Poll},
};

use pin_project_lite::pin_project;

use super::{Identity, IdentityGuard, IdentityKey, SiteKey};

pin_project! {
    /// Installs an identity around every poll of a component's render
    /// future.
    ///
    /// The identity is derived once, at construction, from the identity
    /// installed at that moment: construction happens inside the parent's
    /// body, so the parent is captured even though the future may be polled
    /// later, interleaved with its siblings. Each poll then installs the
    /// derived identity for exactly its duration, so siblings running
    /// concurrently on one task each see their own.
    #[must_use = "futures do nothing unless polled"]
    pub struct IdentityFuture<F> {
        #[pin]
        fut: F,
        identity: Identity,
    }
}

impl<F: Future + Send> IdentityFuture<F> {
    /// Wraps `fut` as a child invocation at `site`.
    pub fn child(site: SiteKey, fut: F) -> impl Future<Output = F::Output> + Send {
        Self {
            fut,
            identity: Identity::current_raw().child(site),
        }
    }

    /// Wraps `fut` as a keyed child invocation at `site`.
    pub fn keyed(
        site: SiteKey,
        key: impl IdentityKey,
        fut: F,
    ) -> impl Future<Output = F::Output> + Send {
        Self {
            fut,
            identity: Identity::current_raw().keyed_child(site, key),
        }
    }

    /// Wraps `fut` as a child invocation at `site` whose repetitions cannot
    /// be told apart, recording `label` as the ambiguity.
    pub fn ambiguous(
        site: SiteKey,
        label: &'static str,
        fut: F,
    ) -> impl Future<Output = F::Output> + Send {
        Self {
            fut,
            identity: Identity::current_raw().ambiguous_child(site, label),
        }
    }
}

impl<F: Future> Future for IdentityFuture<F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, task_cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let _guard = IdentityGuard::install(*this.identity);
        this.fut.poll(task_cx)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        panic::{AssertUnwindSafe, catch_unwind},
        pin::pin,
        task::Waker,
    };

    use super::*;

    const SITE_A: SiteKey = SiteKey::new(file!(), line!(), column!(), 0);
    const SITE_B: SiteKey = SiteKey::new(file!(), line!(), column!(), 0);

    /// Drives `fut` to completion on the current thread.
    ///
    /// The futures under test never wait on external events, so polling in
    /// a tight loop is sufficient.
    fn block_on<F: Future>(fut: F) -> F::Output {
        let mut fut = pin!(fut);
        let mut task = Context::from_waker(Waker::noop());
        loop {
            if let Poll::Ready(output) = fut.as_mut().poll(&mut task) {
                return output;
            }
        }
    }

    /// A future that is pending on its first poll and ready on its second.
    struct YieldOnce(bool);

    impl Future for YieldOnce {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, _task_cx: &mut Context<'_>) -> Poll<()> {
            if self.0 {
                Poll::Ready(())
            } else {
                self.0 = true;
                Poll::Pending
            }
        }
    }

    #[test]
    fn the_future_installs_its_identity_only_while_polling() {
        let fut = IdentityFuture::child(SITE_A, async {
            assert_eq!(Identity::current(), Identity::ROOT.child(SITE_A));
        });
        assert_eq!(Identity::current(), Identity::ROOT);
        block_on(fut);
        assert_eq!(Identity::current(), Identity::ROOT);
    }

    #[test]
    fn the_identity_is_derived_at_construction() {
        let fut = {
            let _parent = IdentityGuard::enter(SITE_A);
            IdentityFuture::child(SITE_B, async { Identity::current() })
        };
        // The parent guard is gone, but the future captured its identity.
        assert_eq!(block_on(fut), Identity::ROOT.child(SITE_A).child(SITE_B));
    }

    #[test]
    fn interleaved_siblings_each_see_their_own_identity() {
        let sibling = |key: u32| {
            IdentityFuture::keyed(SITE_A, key, async move {
                let before = Identity::current();
                YieldOnce(false).await;
                assert_eq!(Identity::current(), before);
                before
            })
        };
        let mut first = pin!(sibling(1));
        let mut second = pin!(sibling(2));
        let mut task = Context::from_waker(Waker::noop());

        // Interleave the two futures across their yield points.
        assert!(first.as_mut().poll(&mut task).is_pending());
        assert!(second.as_mut().poll(&mut task).is_pending());
        let Poll::Ready(first) = first.as_mut().poll(&mut task) else {
            panic!("ready on the second poll");
        };
        let Poll::Ready(second) = second.as_mut().poll(&mut task) else {
            panic!("ready on the second poll");
        };
        assert_ne!(first, second);
        assert_eq!(first, Identity::ROOT.keyed_child(SITE_A, 1));
        assert_eq!(second, Identity::ROOT.keyed_child(SITE_A, 2));
    }

    #[test]
    fn the_future_restores_the_identity_when_a_poll_panics() {
        let mut fut = pin!(IdentityFuture::child(SITE_A, async { panic!("boom") }));
        let result = catch_unwind(AssertUnwindSafe(|| {
            let mut task = Context::from_waker(Waker::noop());
            let _ = fut.as_mut().poll(&mut task);
        }));
        assert!(result.is_err());
        assert_eq!(Identity::current(), Identity::ROOT);
    }

    #[test]
    fn an_ambiguous_future_poisons_its_descendants() {
        let fut = IdentityFuture::ambiguous(SITE_A, "`card` at src/a.rs:1", async {
            let error = Identity::try_current().unwrap_err();
            let keyed =
                IdentityFuture::keyed(SITE_B, 7, async { Identity::try_current().unwrap_err() });
            (error, keyed.await)
        });
        let (outer, inner) = block_on(fut);
        assert_eq!(outer.label(), "`card` at src/a.rs:1");
        assert_eq!(inner.label(), "`card` at src/a.rs:1");
    }
}
