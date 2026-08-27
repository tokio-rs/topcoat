use std::{
    pin::Pin,
    sync::atomic::{AtomicU64, Ordering},
    task::{Context, Poll},
};

use pin_project_lite::pin_project;
use topcoat_core::error::{Error, Result};

use super::drive::{Emission, collect};
use crate::{RegionId, Swap, View, ViewBuffer, buffer::ViewHandle};

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
        // The region's id, decided at the first poll.
        region: Option<RegionId>,
        // An error the body completed with while an emission was still in
        // flight; yielded through `poll_swap` after it.
        error: Option<Error>,
        done: bool,
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
            done: false,
        }
    }
}

impl<Fut> View for LiveView<'_, Fut>
where
    Fut: Future<Output = Result<()>> + Send,
{
    fn poll_first(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<ViewHandle>> {
        let this = self.project();
        let region = *this
            .region
            .get_or_insert_with(|| RegionId(NEXT_REGION.fetch_add(1, Ordering::Relaxed)));
        let (poll, emitted) = collect(this.body, cx);
        if let Some(emission) = emitted {
            match poll {
                Poll::Ready(Ok(())) => *this.done = true,
                Poll::Ready(Err(error)) => *this.error = Some(error),
                Poll::Pending => {}
            }
            return match emission {
                Emission::Content(content) => {
                    let view = this.buf.block(|parts| {
                        parts.push_str_unescaped(&format!("<!--tc:{}-->", region.0));
                        parts.push_view_handle(content);
                        parts.push_str_unescaped(&format!("<!--/tc:{}-->", region.0));
                    });
                    Poll::Ready(Ok(view))
                }
                Emission::Swap(_) => {
                    panic!("a live region emitted a swap before its first content")
                }
            };
        }
        match poll {
            Poll::Pending => Poll::Pending,
            // The body completed without emitting; the region renders
            // nothing and can never update, so no markers are written.
            Poll::Ready(Ok(())) => {
                *this.done = true;
                Poll::Ready(Ok(ViewHandle::empty()))
            }
            Poll::Ready(Err(error)) => {
                *this.done = true;
                Poll::Ready(Err(error))
            }
        }
    }

    fn poll_swap(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Swap>>> {
        let this = self.project();
        if let Some(error) = this.error.take() {
            *this.done = true;
            return Poll::Ready(Some(Err(error)));
        }
        if *this.done {
            return Poll::Ready(None);
        }
        let region = this
            .region
            .expect("`poll_swap` called before `poll_first` returned `Ready`");
        let (poll, emitted) = collect(this.body, cx);
        if let Some(emission) = emitted {
            match poll {
                Poll::Ready(Ok(())) => *this.done = true,
                Poll::Ready(Err(error)) => *this.error = Some(error),
                Poll::Pending => {}
            }
            return Poll::Ready(Some(Ok(match emission {
                Emission::Content(replacement) => Swap {
                    region,
                    replacement,
                },
                // A nested region's swap targets its own region; it passes
                // through untouched.
                Emission::Swap(swap) => swap,
            })));
        }
        match poll {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(())) => {
                *this.done = true;
                Poll::Ready(None)
            }
            Poll::Ready(Err(error)) => {
                *this.done = true;
                Poll::Ready(Some(Err(error)))
            }
        }
    }
}
