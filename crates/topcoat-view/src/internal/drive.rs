use std::{
    cell::Cell,
    pin::Pin,
    task::{Context, Poll, ready},
};

use pin_project_lite::pin_project;
use topcoat_core::error::Result;

use crate::{Swap, View, ViewBuffer, ViewHandle};

thread_local! {
    /// The tunnel between a collecting poll and the drive inside it, on the
    /// current task.
    ///
    /// A [`MoveView`](super::MoveView) or [`LiveView`](super::LiveView)
    /// polls its body through [`collect`], which opens the tunnel for the
    /// duration of the poll. A [`DriveView`] awaited by that body places
    /// what it produced in the tunnel right before it returns `Pending`, and
    /// the collecting poll takes it out right after the body's poll returns.
    /// Nested collectors park the enclosing tunnel for their own poll, so a
    /// collector only ever sees the emissions of its own body.
    static TUNNEL: Cell<Tunnel> = const { Cell::new(Tunnel::Closed) };
}

/// The state of the tunnel on the current task.
enum Tunnel {
    /// No collecting poll is running.
    Closed,
    /// A collecting poll is running and nothing was emitted yet.
    Open,
    /// A collecting poll is running and an emission awaits collection.
    Emitted(Emission),
}

/// One item a drive tunnels to its collecting poll.
pub(super) enum Emission {
    /// The driven view's first content.
    Content(ViewHandle),
    /// A swap the driven view emitted after its first content, passed
    /// through verbatim.
    Swap(Swap),
}

/// Restores the tunnel state a [`collect`] parked, also when the collected
/// poll panics.
struct Restore {
    previous: Tunnel,
}

impl Drop for Restore {
    fn drop(&mut self) {
        TUNNEL.set(std::mem::replace(&mut self.previous, Tunnel::Closed));
    }
}

/// Polls `body` with the tunnel open and returns the emission a drive
/// inside it placed, if any.
pub(super) fn collect<F>(
    body: Pin<&mut F>,
    cx: &mut Context<'_>,
) -> (Poll<F::Output>, Option<Emission>)
where
    F: Future + ?Sized,
{
    let _restore = Restore {
        previous: TUNNEL.replace(Tunnel::Open),
    };
    let poll = body.poll(cx);
    let emission = match TUNNEL.replace(Tunnel::Closed) {
        Tunnel::Emitted(emission) => Some(emission),
        Tunnel::Open | Tunnel::Closed => None,
    };
    (poll, emission)
}

/// Moves the emission into the tunnel if it is open and free.
///
/// The tunnel is occupied when another emission of the same poll still
/// awaits collection; the emission then stays put and the caller stays
/// pending, trying again when polled next.
///
/// # Panics
///
/// Panics if no collecting poll is running: the drive is awaited outside
/// the body of the `view!` template or `live!` region it belongs to.
fn try_yield(emission: &mut Option<Emission>) {
    match TUNNEL.replace(Tunnel::Closed) {
        Tunnel::Open => TUNNEL.set(Tunnel::Emitted(
            emission.take().expect("an emission awaits yielding"),
        )),
        occupied @ Tunnel::Emitted(_) => TUNNEL.set(occupied),
        Tunnel::Closed => panic!(
            "a view was driven outside the body of the `view!` template or `live!` region \
             polling it"
        ),
    }
}

/// Returns the future a body awaits to poll `view` in place.
///
/// Each poll forwards to the view: first `poll_first`, whose content is
/// tunneled to the collecting poll as the body's own, then `poll_swap` for
/// every swap after it, each tunneled through verbatim. The future stays
/// pending after each emission, so the view lives on in the body for the
/// next poll, and resolves once the view has no further updates. An error
/// the view produces is returned to the body instead of being tunneled.
///
/// # Panics
///
/// Panics when polled outside the poll of the `view!` template or `live!`
/// region whose body awaits it.
#[doc(hidden)]
pub fn drive<V: View>(view: V) -> DriveView<'static, V> {
    DriveView {
        view,
        seal: None,
        first: true,
        pending: None,
    }
}

/// Returns the future a body awaits to poll `view` in place and seal its
/// first content into `buf`, the buffer the view was built with.
///
/// Like [`drive`], except that the content tunneled out is self-contained,
/// for a template that builds in a buffer of its own.
#[doc(hidden)]
pub fn drive_sealed<V: View>(buf: &ViewBuffer, view: V) -> DriveView<'_, V> {
    DriveView {
        view,
        seal: Some(buf),
        first: true,
        pending: None,
    }
}

pin_project! {
    /// The future behind [`drive`] and [`drive_sealed`].
    #[must_use = "futures do nothing unless polled"]
    pub struct DriveView<'a, V> {
        #[pin]
        view: V,
        // The buffer the first content is sealed into, if any.
        seal: Option<&'a ViewBuffer>,
        first: bool,
        // An emission waiting for the tunnel to be free.
        pending: Option<Emission>,
    }
}

impl<V> Future for DriveView<'_, V>
where
    V: View,
{
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        loop {
            if this.pending.is_some() {
                try_yield(this.pending);
                // Placed or not, the emission awaits collection by the
                // enclosing poll; resume when polled again.
                return Poll::Pending;
            }
            if *this.first {
                let content = ready!(this.view.as_mut().poll_first(cx))?;
                *this.first = false;
                let content = match this.seal {
                    Some(buf) => content.seal(buf),
                    None => content,
                };
                *this.pending = Some(Emission::Content(content));
            } else {
                match ready!(this.view.as_mut().poll_swap(cx)) {
                    Some(Ok(swap)) => *this.pending = Some(Emission::Swap(swap)),
                    Some(Err(error)) => return Poll::Ready(Err(error)),
                    None => return Poll::Ready(Ok(())),
                }
            }
        }
    }
}
