use std::{
    pin::Pin,
    task::{Context, Poll},
};

use pin_project_lite::pin_project;
use topcoat_core::error::Result;

use super::{Identity, IdentityGuard};
use crate::{View, ViewFirst, ViewSwap};

pin_project! {
    /// Installs an identity around every poll of a view.
    ///
    /// A component invocation becomes one: its body future and the view it
    /// resolves to both poll through here, so everything the invocation
    /// runs, from the body itself down to the templates it builds and
    /// drives later, sees the invocation's identity. Each poll installs the
    /// identity for exactly its duration, so siblings interleaving on one
    /// task each see their own.
    pub struct IdentityView<V> {
        #[pin]
        view: V,
        identity: Identity,
    }
}

impl<V: View> IdentityView<V> {
    /// Wraps `view` to poll at `identity`, for example one an
    /// [`IdentityGuard`] already installed while the view was built.
    pub fn new(identity: Identity, view: V) -> Self {
        Self { view, identity }
    }
}

impl<V: View> View for IdentityView<V> {
    fn poll_first(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<ViewFirst>> {
        let this = self.project();
        let _guard = IdentityGuard::install(*this.identity);
        this.view.poll_first(cx)
    }

    fn poll_swap(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Option<ViewSwap>>> {
        let this = self.project();
        let _guard = IdentityGuard::install(*this.identity);
        this.view.poll_swap(cx)
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
    use crate::{ViewHandle, identity::SiteKey};

    const SITE_A: SiteKey = SiteKey::new(file!(), line!(), column!(), 0);

    /// A view that records the identity installed while it polls.
    ///
    /// The first poll of each method is pending, so a test can interleave
    /// two of them across a yield point; the second resolves.
    struct Probe {
        polled: bool,
        seen: Vec<Identity>,
    }

    impl Probe {
        fn new() -> Self {
            Self {
                polled: false,
                seen: Vec::new(),
            }
        }
    }

    impl View for Probe {
        fn poll_first(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<ViewFirst>> {
            self.seen.push(Identity::current());
            if std::mem::replace(&mut self.polled, true) {
                Poll::Ready(Ok(ViewFirst {
                    content: ViewHandle::empty(),
                    live: true,
                }))
            } else {
                Poll::Pending
            }
        }

        fn poll_swap(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<Option<ViewSwap>>> {
            self.seen.push(Identity::current());
            Poll::Ready(Ok(None))
        }
    }

    /// A view that panics when polled.
    struct Boom;

    impl View for Boom {
        fn poll_first(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<ViewFirst>> {
            panic!("boom")
        }

        fn poll_swap(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<Option<ViewSwap>>> {
            panic!("boom")
        }
    }

    #[test]
    fn the_view_installs_its_identity_only_while_polling() {
        let identity = Identity::ROOT.child(SITE_A);
        let mut view = pin!(IdentityView::new(identity, Probe::new()));
        let mut cx = Context::from_waker(Waker::noop());

        assert_eq!(Identity::current(), Identity::ROOT);
        assert!(view.as_mut().poll_first(&mut cx).is_pending());
        assert!(view.as_mut().poll_first(&mut cx).is_ready());
        assert!(view.as_mut().poll_swap(&mut cx).is_ready());
        assert_eq!(Identity::current(), Identity::ROOT);
        assert_eq!(view.view.seen, [identity, identity, identity]);
    }

    #[test]
    fn interleaved_siblings_each_see_their_own_identity() {
        let first_identity = Identity::ROOT.keyed_child(SITE_A, 1);
        let second_identity = Identity::ROOT.keyed_child(SITE_A, 2);
        let mut first = pin!(IdentityView::new(first_identity, Probe::new()));
        let mut second = pin!(IdentityView::new(second_identity, Probe::new()));
        let mut cx = Context::from_waker(Waker::noop());

        // Interleave the two views across their pending polls.
        assert!(first.as_mut().poll_first(&mut cx).is_pending());
        assert!(second.as_mut().poll_first(&mut cx).is_pending());
        assert!(first.as_mut().poll_first(&mut cx).is_ready());
        assert!(second.as_mut().poll_first(&mut cx).is_ready());

        assert_eq!(first.view.seen, [first_identity, first_identity]);
        assert_eq!(second.view.seen, [second_identity, second_identity]);
    }

    #[test]
    fn the_view_restores_the_identity_when_a_poll_panics() {
        let mut view = pin!(IdentityView::new(Identity::ROOT.child(SITE_A), Boom));
        let result = catch_unwind(AssertUnwindSafe(|| {
            let mut cx = Context::from_waker(Waker::noop());
            let _ = view.as_mut().poll_first(&mut cx);
        }));
        assert!(result.is_err());
        assert_eq!(Identity::current(), Identity::ROOT);
    }
}
