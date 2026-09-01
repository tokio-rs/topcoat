use std::{
    cell::Cell,
    future::Ready,
    mem,
    pin::Pin,
    sync::atomic::{AtomicU64, Ordering},
    task::{Context, Poll},
};

use pin_project_lite::pin_project;
use topcoat_core::error::Result;

use crate::{EmitToken, RegionId, View, ViewBufferScope, ViewFirst, ViewSwap};

static NEXT_REGION: AtomicU64 = AtomicU64::new(1);

pin_project! {
    pub struct LiveView<Fut> {
        #[pin]
        body: Fut,
        region: Option<RegionId>,
        stash: Option<ViewSwap>,
    }
}

impl<Fut> LiveView<Fut>
where
    Fut: Future<Output = Result<EmitToken>>,
{
    #[doc(hidden)]
    pub fn new(body: Fut) -> Self {
        Self {
            body,
            region: None,
            stash: None,
        }
    }
}

impl LiveView<Ready<Result<EmitToken>>> {
    pub fn drive<V: View>(view: V) -> impl Future<Output = Result<EmitToken>> {
        DriveFuture { view, first: true }
    }
}

impl<Fut> View for LiveView<Fut>
where
    Fut: Future<Output = Result<EmitToken>> + Send,
{
    fn poll_first(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<ViewFirst>> {
        let mut this = self.project();
        let region = *this
            .region
            .get_or_insert_with(|| RegionId(NEXT_REGION.fetch_add(1, Ordering::Relaxed)));

        let (poll, yielded) = {
            let _guard = YieldGuard::new();
            (this.body.as_mut().poll(cx), YIELD.take())
        };

        match (poll, yielded) {
            (Poll::Pending, Yield::First(first)) => {
                // Poll again to determine liveness. If the second poll returns pending, we
                // expect this view to yield again in the future.
                let poll = {
                    let _guard = YieldGuard::new();
                    let poll = this.body.poll(cx);
                    *this.stash = YIELD.take().into_swap(region);
                    poll
                };

                if let Poll::Ready(Err(e)) = poll {
                    return Poll::Ready(Err(e));
                }

                let live = poll.is_pending();
                if !live {
                    // The body is done, so nothing will replace this content and it needs no
                    // markers.
                    return Poll::Ready(Ok(ViewFirst {
                        content: first.content,
                        live,
                    }));
                }

                let first = ViewFirst {
                    content: ViewBufferScope::with(|buffer| {
                        buffer.block(|parts| {
                            parts.push_comment(|parts| {
                                parts.push_promoted_str_unescaped(&"tc:");
                                parts.push_u64(region.0);
                            });
                            parts.push_view_handle(first.content);
                            parts.push_comment(|parts| {
                                parts.push_promoted_str_unescaped(&"/tc:");
                                parts.push_u64(region.0);
                            });
                        })
                    }),
                    live,
                };
                Poll::Ready(Ok(first))
            }
            (Poll::Pending, Yield::NotSet) => Poll::Pending,
            (Poll::Pending, Yield::Swap(_)) => {
                panic!("live view future yielded a swap before its first content")
            }
            (Poll::Ready(_), Yield::First(_) | Yield::Swap(_)) => {
                panic!("live view future yielded without returning pending")
            }
            (Poll::Ready(Err(e)), Yield::NotSet) => Poll::Ready(Err(e)),
            (Poll::Ready(Ok(_)), Yield::NotSet) => {
                panic!("live view future completed without yielding anything")
            }
        }
    }

    fn poll_swap(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Option<ViewSwap>>> {
        let this = self.project();

        if let Some(stash) = this.stash.take() {
            return Poll::Ready(Ok(Some(stash)));
        }

        let region = (*this.region).expect("live view polled for a swap before its first content");

        let (poll, yielded) = {
            let _guard = YieldGuard::new();
            (this.body.poll(cx), YIELD.take())
        };

        match (poll, yielded.into_swap(region)) {
            (Poll::Pending, Some(swap)) => Poll::Ready(Ok(Some(swap))),
            (Poll::Pending, None) => Poll::Pending,
            (Poll::Ready(_), Some(_)) => {
                panic!("live view future yielded without returning pending")
            }
            (Poll::Ready(Err(e)), None) => Poll::Ready(Err(e)),
            (Poll::Ready(Ok(_)), None) => Poll::Ready(Ok(None)),
        }
    }
}

pin_project! {
    struct DriveFuture<V> {
        #[pin]
        view: V,
        first: bool,
    }
}

impl<V> Future for DriveFuture<V>
where
    V: View,
{
    type Output = Result<EmitToken>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        if *this.first {
            match this.view.poll_first(cx) {
                Poll::Ready(Ok(first)) => {
                    *this.first = false;
                    YIELD.set(Yield::First(first));
                    Poll::Pending
                }
                Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
                Poll::Pending => Poll::Pending,
            }
        } else {
            match this.view.poll_swap(cx) {
                Poll::Ready(Ok(Some(swap))) => {
                    YIELD.set(Yield::Swap(swap));
                    Poll::Pending
                }
                Poll::Ready(Ok(None)) => Poll::Ready(Ok(EmitToken)),
                Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
                Poll::Pending => Poll::Pending,
            }
        }
    }
}

thread_local! {
    /// What the view driven on this task handed back on its last poll.
    ///
    /// A driven view reports out of band because it is polled through a
    /// future, which has no room for a value in its pending state. The slot
    /// is read back by the poll that set it going, so it only ever holds a
    /// value across a single poll.
    static YIELD: Cell<Yield> = const { Cell::new(Yield::NotSet) };
}

/// The value a driven view handed back, if any.
#[derive(Default)]
enum Yield {
    /// The view has not reported since the slot was last read.
    #[default]
    NotSet,
    /// The view's first content.
    First(ViewFirst),
    /// An update to content the view already reported.
    Swap(ViewSwap),
}

impl Yield {
    /// Turns what a driven view handed back into a swap of `region`.
    ///
    /// A region that already emitted has its markers in the document, so
    /// first content arriving after that replaces what sits between them.
    /// A swap already names the region it belongs to, which is a nested one
    /// when the emitted content is live in its own right.
    fn into_swap(self, region: RegionId) -> Option<ViewSwap> {
        match self {
            Self::NotSet => None,
            Self::First(first) => Some(ViewSwap {
                region,
                replacement: first.content,
            }),
            Self::Swap(swap) => Some(swap),
        }
    }
}

/// Keeps the yield slot of an enclosing poll while a nested one runs.
struct YieldGuard {
    prev: Yield,
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
