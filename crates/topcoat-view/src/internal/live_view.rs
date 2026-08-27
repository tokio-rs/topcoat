use std::{
    pin::Pin,
    sync::atomic::{AtomicU64, Ordering},
    task::{Context, Poll},
};

use pin_project_lite::pin_project;
use topcoat_core::error::Result;

use super::drive::{Emission, collect};
use crate::{RegionId, Step, Swap, View, ViewBuffer, buffer::ViewHandle};

/// The id of the next live region.
///
/// A process-wide counter, so every live region in a response is distinct
/// without threading state through the request.
static NEXT_REGION: AtomicU64 = AtomicU64::new(1);

pin_project! {
    /// A live region: a node position whose content is replaced by the views
    /// its body emits.
    ///
    /// The `live!` macro wraps its body in this type. The body emits with
    /// `emit!`, which drives a self-contained view in place: the first
    /// emission becomes the region's content, and every later one becomes a
    /// [`Swap`] replacing that content on the client. The content is
    /// surrounded by marker comments in the buffer the region was built
    /// with, unless the body completes along with it: a region that emits
    /// once renders as plain content and never updates.
    pub struct LiveView<'a, Fut> {
        buf: &'a ViewBuffer,
        #[pin]
        body: Fut,
        // The region's id, decided when its first content is emitted with
        // the body still running.
        region: Option<RegionId>,
    }
}

impl<'a, Fut> LiveView<'a, Fut>
where
    Fut: Future<Output = Result<()>>,
{
    #[doc(hidden)]
    pub fn new(buf: &'a ViewBuffer, body: Fut) -> Self {
        Self {
            buf,
            body,
            region: None,
        }
    }
}

impl<Fut> View for LiveView<'_, Fut>
where
    Fut: Future<Output = Result<()>> + Send,
{
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Step>> {
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
                let content = this.buf.block(|parts| {
                    parts.push_str_unescaped(&format!("<!--tc:{}-->", region.0));
                    parts.push_view_handle(content);
                    parts.push_str_unescaped(&format!("<!--/tc:{}-->", region.0));
                });
                Step::Content { content, live }
            }
            (Emission::Content(replacement), Some(region)) => Step::Swap {
                swap: Swap {
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
