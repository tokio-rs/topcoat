use std::{
    pin::Pin,
    sync::atomic::{AtomicU64, Ordering},
    task::{Context, Poll},
};

use pin_project_lite::pin_project;
use topcoat_core::error::{Error, Result};

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
    /// emission becomes the region's content, surrounded by marker comments
    /// in the buffer the region was built with, and every later one becomes
    /// a [`Swap`] replacing that content on the client.
    pub struct LiveView<'a, Fut> {
        buf: &'a ViewBuffer,
        #[pin]
        body: Fut,
        // The region's id, decided when its first content is emitted.
        region: Option<RegionId>,
        // An error the body completed with while an emission was still in
        // flight; yielded by the poll after it.
        error: Option<Error>,
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
            error: None,
        }
    }
}

impl<Fut> View for LiveView<'_, Fut>
where
    Fut: Future<Output = Result<()>> + Send,
{
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Step>> {
        let this = self.project();
        if let Some(error) = this.error.take() {
            return Poll::Ready(Err(error));
        }
        let (poll, emitted) = collect(this.body, cx);
        if let Some(emission) = emitted {
            let live = match poll {
                Poll::Ready(Ok(())) => false,
                Poll::Ready(Err(error)) => {
                    *this.error = Some(error);
                    true
                }
                Poll::Pending => true,
            };
            return Poll::Ready(Ok(match (emission, *this.region) {
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
            }));
        }
        match poll {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(())) if this.region.is_some() => Poll::Ready(Ok(Step::Done)),
            // The body completed without emitting; the region renders
            // nothing and can never update, so no markers are written.
            Poll::Ready(Ok(())) => Poll::Ready(Ok(Step::Content {
                content: ViewHandle::empty(),
                live: false,
            })),
            Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
        }
    }
}
