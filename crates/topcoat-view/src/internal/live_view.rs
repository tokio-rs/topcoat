use std::{
    pin::Pin,
    sync::atomic::{AtomicU64, Ordering},
    task::{Context, Poll},
};

use pin_project_lite::pin_project;
use topcoat_core::error::Result;

use crate::{
    RegionId, View, ViewSwap,
    buffer::{ViewBufferScope, ViewHandle},
};

static NEXT_REGION: AtomicU64 = AtomicU64::new(1);

pin_project! {
    pub struct LiveView<Fut> {
        #[pin]
        body: Fut,
        region: Option<RegionId>,
    }
}

impl<Fut> LiveView<Fut>
where
    Fut: Future<Output = Result<()>>,
{
    #[doc(hidden)]
    pub fn new(body: Fut) -> Self {
        Self { body, region: None }
    }
}

impl<Fut> View for LiveView<Fut>
where
    Fut: Future<Output = Result<()>> + Send,
{
    fn poll_first(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Step>> {
        let this = self.project();
        let (poll, emitted) = collect(this.body, cx);
        // An error the body completes with takes precedence over the
        // emission in flight, which is dropped.
        let live = match poll {
            Poll::Pending => true,
            Poll::Ready(Ok(())) => false,
            Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
        };
        let Some(emission) = emitted else {
            return match (poll, *this.region) {
                (Poll::Pending, _) => Poll::Pending,
                (Poll::Ready(_), Some(_)) => Poll::Ready(Ok(Step::Done)),
                // The body completed without emitting; the region renders
                // nothing and can never update.
                (Poll::Ready(_), None) => Poll::Ready(Ok(Step::Content {
                    content: ViewHandle::empty(),
                    live: false,
                })),
            };
        };
        Poll::Ready(Ok(match (emission, *this.region) {
            (Emission::Content(content), None) if !live => Step::Content { content, live },
            (Emission::Content(content), None) => {
                let region = RegionId(NEXT_REGION.fetch_add(1, Ordering::Relaxed));
                *this.region = Some(region);
                let content = ViewBufferScope::block(|parts| {
                    parts.push_str_unescaped(&format!("<!--tc:{}-->", region.0));
                    parts.push_view_handle(content);
                    parts.push_str_unescaped(&format!("<!--/tc:{}-->", region.0));
                });
                Step::Content { content, live }
            }
            (Emission::Content(replacement), Some(region)) => Step::Swap {
                swap: ViewSwap {
                    region,
                    replacement,
                },
                live,
            },
            // A nested region's swap targets its own region; it passes
            // through untouched.
            (Emission::Swap(swap), Some(_)) => Step::Swap { swap, live },
            (Emission::Swap(_), None) => {
                panic!("a live region emitted a swap before its first content")
            }
        }))
    }
}
