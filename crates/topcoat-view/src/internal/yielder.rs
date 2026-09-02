use std::{
    cell::Cell,
    mem,
    pin::Pin,
    task::{Context, Poll},
};

use pin_project_lite::pin_project;
use topcoat_core::error::Result;

use crate::{RegionId, View, ViewFirst, ViewSwap};

/// Polls `body` and takes what the view driven inside it handed back.
///
/// The body gets a clean slot for the duration of the poll, so a driven
/// view nested in it reports to this poll and not to an enclosing one.
pub(super) fn poll_body<Fut: Future>(
    body: Pin<&mut Fut>,
    cx: &mut Context<'_>,
) -> (Poll<Fut::Output>, Option<Yield>) {
    let _guard = YieldGuard::new();
    (body.poll(cx), YIELD.take())
}

thread_local! {
    /// What the view driven on this task handed back on its last poll.
    ///
    /// A driven view reports out of band because it is polled through a
    /// future, which has no room for a value in its pending state. The slot
    /// is read back by the poll that set it going, so it only ever holds a
    /// value across a single poll.
    static YIELD: Cell<Option<Yield>> = const { Cell::new(None) };
}

/// The value a driven view handed back to the poll that set it going.
pub(super) enum Yield {
    /// The view's first content.
    First(ViewFirst),
    /// An update to content the view already reported.
    Swap(ViewSwap),
}

impl Yield {
    /// Turns the value into a swap of `region`.
    ///
    /// A region that already emitted has its markers in the document, so
    /// first content arriving after that replaces what sits between them.
    /// A swap already names the region it belongs to, which is a nested one
    /// when the emitted content is live in its own right.
    pub(super) fn into_swap(self, region: RegionId) -> ViewSwap {
        match self {
            Self::First(first) => ViewSwap {
                region,
                replacement: first.content,
            },
            Self::Swap(swap) => swap,
        }
    }

    /// Places the value in the slot of the current poll.
    ///
    /// The slot holds a single value, so when another drive filled it
    /// during the same poll the value comes back to be offered again on a
    /// later one.
    fn offer(self) -> Option<Self> {
        YIELD.with(|slot| match slot.take() {
            None => {
                slot.set(Some(self));
                None
            }
            Some(taken) => {
                slot.set(Some(taken));
                Some(self)
            }
        })
    }
}

/// Keeps the yield slot of an enclosing poll while a nested one runs.
struct YieldGuard {
    prev: Option<Yield>,
}

impl YieldGuard {
    fn new() -> Self {
        Self { prev: YIELD.take() }
    }
}

impl Drop for YieldGuard {
    fn drop(&mut self) {
        YIELD.set(mem::take(&mut self.prev));
    }
}

pin_project! {
    /// The future a body awaits to drive a view in place.
    ///
    /// The view's first content and every swap after it are handed to the
    /// enclosing poll, and the future resolves once the view has no further
    /// updates.
    ///
    /// A poll carries a single value. When a sibling drive already handed
    /// one back during the same poll, this one holds on to its own and
    /// offers it again on a later poll, so drives that run concurrently
    /// under a combinator like `join!` lose nothing.
    pub(super) struct DriveFuture<V> {
        #[pin]
        view: V,
        first: bool,
        deferred: Option<Yield>,
    }
}

impl<V: View> DriveFuture<V> {
    pub(super) fn new(view: V) -> Self {
        Self {
            view,
            first: true,
            deferred: None,
        }
    }
}

impl<V: View> Future for DriveFuture<V> {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        let value = match this.deferred.take() {
            Some(deferred) => deferred,
            None if *this.first => match this.view.poll_first(cx) {
                Poll::Ready(Ok(first)) => {
                    *this.first = false;
                    Yield::First(first)
                }
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            },
            None => match this.view.poll_swap(cx) {
                Poll::Ready(Ok(Some(swap))) => Yield::Swap(swap),
                Poll::Ready(Ok(None)) => return Poll::Ready(Ok(())),
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            },
        };

        *this.deferred = value.offer();
        if this.deferred.is_some() {
            // The enclosing poll has a value to deliver and runs again for it.
            // The wake covers combinators that only poll futures which asked
            // to be polled.
            cx.waker().wake_by_ref();
        }
        Poll::Pending
    }
}
