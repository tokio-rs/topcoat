use std::{
    cell::Cell,
    pin::Pin,
    sync::atomic::{AtomicU64, Ordering},
    task::{Context, Poll},
};

use pin_project_lite::pin_project;
use topcoat_core::{context::Cx, error::Result};

use crate::{RegionId, View, ViewBufferScope, ViewFirst, ViewSwap};

static NEXT_REGION: AtomicU64 = AtomicU64::new(1);

pin_project! {
    pub struct LiveView<'cx, Fut> {
        cx: &'cx Cx,
        #[pin]
        body: Fut,
        region: Option<RegionId>,
        stash: Option<ViewSwap>,
    }
}

impl<'cx, Fut> LiveView<'cx, Fut>
where
    Fut: Future<Output = Result<()>>,
{
    #[doc(hidden)]
    pub fn new(cx: &'cx Cx, body: Fut) -> Self {
        Self {
            cx,
            body,
            region: None,
            stash: None,
        }
    }
}

impl<Fut> LiveView<'_, Fut> {
    pub fn drive<V: View>(view: V) -> DriveFuture<V> {
        DriveFuture { view, first: true }
    }
}

impl<Fut> View for LiveView<'_, Fut>
where
    Fut: Future<Output = Result<()>> + Send,
{
    fn poll_first(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<ViewFirst>> {
        let mut this = self.project();
        let region = *this
            .region
            .get_or_insert_with(|| RegionId(NEXT_REGION.fetch_add(1, Ordering::Relaxed)));

        let (poll, yielded) = {
            let _guard = FirstGuard::new();
            (this.body.as_mut().poll(cx), FIRST_YIELD.take())
        };

        match (poll, yielded) {
            (Poll::Pending, Some(value)) => {
                // Poll again to determine liveness. If the second poll returns pending, we
                // expect this view to yield again in the future.
                let poll = {
                    let _guard = FirstGuard::new();
                    let poll = this.body.poll(cx);
                    *this.stash = SWAP_YIELD.take();
                    poll
                };

                if let Poll::Ready(Err(e)) = poll {
                    return Poll::Ready(Err(e));
                }

                let first = ViewFirst {
                    content: ViewBufferScope::with(|buffer| {
                        buffer.block(|parts| {
                            parts.push_comment(|parts| {
                                parts.push_promoted_str_unescaped(&"tc:");
                                parts.push_u64(region.0);
                            });
                            parts.push_view_handle(value.content);
                            parts.push_comment(|parts| {
                                parts.push_promoted_str_unescaped(&"/tc:");
                                parts.push_u64(region.0);
                            });
                        })
                    }),
                    live: poll.is_pending(),
                };
                Poll::Ready(Ok(first))
            }
            (Poll::Pending, None) => Poll::Pending,
            (Poll::Ready(_), Some(_)) => {
                panic!("live view future yielded without returning pending")
            }
            (Poll::Ready(Err(e)), None) => Poll::Ready(Err(e)),
            (Poll::Ready(Ok(())), None) => {
                panic!("live view future completed without yielding anything")
            }
        }
    }

    fn poll_swap(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Option<ViewSwap>>> {
        let this = self.project();

        if let Some(stash) = this.stash.take() {
            return Poll::Ready(Ok(Some(stash)));
        }

        let (poll, yielded) = {
            let _guard = SwapGuard::new();
            (this.body.poll(cx), SWAP_YIELD.take())
        };

        match (poll, yielded) {
            (Poll::Pending, Some(value)) => Poll::Ready(Ok(Some(value))),
            (Poll::Pending, None) => Poll::Pending,
            (Poll::Ready(_), Some(_)) => {
                panic!("move view future yielded without returning pending")
            }
            (Poll::Ready(Err(e)), None) => Poll::Ready(Err(e)),
            (Poll::Ready(Ok(())), None) => Poll::Ready(Ok(None)),
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
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        if *this.first {
            match this.view.poll_first(cx) {
                Poll::Ready(Ok(first)) => {
                    *this.first = false;
                    FIRST_YIELD.set(Some(first));
                    Poll::Pending
                }
                Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
                Poll::Pending => Poll::Pending,
            }
        } else {
            match this.view.poll_swap(cx) {
                Poll::Ready(Ok(swap)) => {
                    SWAP_YIELD.set(swap);
                    Poll::Pending
                }
                Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
                Poll::Pending => Poll::Pending,
            }
        }
    }
}

thread_local! {
    static FIRST_YIELD: Cell<Option<ViewFirst>> = const { Cell::new(None) };
}

struct FirstGuard {
    prev: Option<ViewFirst>,
}

impl FirstGuard {
    fn new() -> Self {
        Self {
            prev: FIRST_YIELD.take(),
        }
    }
}

impl Drop for FirstGuard {
    fn drop(&mut self) {
        FIRST_YIELD.replace(self.prev.take());
    }
}

thread_local! {
    static SWAP_YIELD: Cell<Option<ViewSwap>> = const { Cell::new(None) };
}

struct SwapGuard {
    prev: Option<ViewSwap>,
}

impl SwapGuard {
    fn new() -> Self {
        Self {
            prev: SWAP_YIELD.take(),
        }
    }
}

impl Drop for SwapGuard {
    fn drop(&mut self) {
        SWAP_YIELD.replace(self.prev.take());
    }
}
