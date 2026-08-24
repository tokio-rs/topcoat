use std::{
    future::poll_fn,
    marker::PhantomData,
    pin::pin,
    sync::atomic::{AtomicU64, Ordering},
};

use topcoat_core::{context::Cx, error::Result};

use crate::{
    NodeViewPartsStream, NodeWriter, View, ViewChunk, buffer::ViewBufferScope, yielder::yield_,
};

/// The id of the next live position.
///
/// A process-wide counter, so every live position in a response is distinct
/// without threading state through the request.
static NEXT_POSITION: AtomicU64 = AtomicU64::new(1);

/// A live node position: a body that emits a view more than once.
///
/// The `live!` macro wraps its body in this type. The body receives a
/// [`LiveSink`] and emits through it: the first emission becomes the
/// position's content, and every later one becomes a [`ViewChunk::Swap`]
/// replacing that content on the client.
pub struct Live<F, Fut> {
    f: F,
    // Names the body's future type, so it participates in the lifetime
    // bounds of the trait method's returned future.
    future: PhantomData<fn() -> Fut>,
}

impl<F, Fut> Live<F, Fut>
where
    F: FnOnce(LiveSink) -> Fut,
    Fut: Future<Output = Result<()>>,
{
    #[doc(hidden)]
    pub fn new(f: F) -> Self {
        Self {
            f,
            future: PhantomData,
        }
    }
}

impl<F, Fut> NodeViewPartsStream for Live<F, Fut>
where
    F: FnOnce(LiveSink) -> Fut + Send,
    Fut: Future<Output = Result<()>> + Send,
{
    const MULTI: bool = true;

    async fn into_view_parts_stream<'cx>(self, _cx: &'cx Cx, writer: NodeWriter) -> Result<()>
    where
        Self: 'cx,
    {
        let sink = LiveSink {
            id: NEXT_POSITION.fetch_add(1, Ordering::Relaxed),
            writer,
            first: true,
        };
        (self.f)(sink).await
    }
}

/// The emission handle of a live position, passed to a `live!` body.
///
/// The `emit!` macro expands to [`emit`](Self::emit) calls on the sink the
/// enclosing `live!` invocation bound.
pub struct LiveSink {
    /// The position's id: written into the marker comments surrounding its
    /// content, and targeted by the swaps it emits.
    id: u64,
    writer: NodeWriter,
    first: bool,
}

impl LiveSink {
    /// Emits a view at this position.
    ///
    /// Drives `view` to its content. The first emission becomes the
    /// position's content, surrounded by marker comments; every later one
    /// yields a [`ViewChunk::Swap`] replacing the content between the
    /// markers on the client.
    ///
    /// An error the view produces while rendering is returned to the caller
    /// instead of failing the position, so the body can recover; nothing is
    /// emitted for that call.
    ///
    /// The view is dropped once its content resolves, so live positions
    /// nested inside an emitted view are cut off after their own content.
    // TODO: keep driving nested live positions instead of cutting them off
    // once their swaps have somewhere to go.
    pub async fn emit(&mut self, view: impl View) -> Result<()> {
        let mut view = pin!(view);
        // The view polls with the enclosing build's buffer parked, so it
        // roots a buffer of its own and its content chunk leaves it sealed
        // and self-contained.
        let content = loop {
            let chunk = poll_fn(|task| {
                let mut parked = None;
                let _scope = ViewBufferScope::swap(&mut parked);
                view.as_mut().poll_next(task)
            })
            .await;
            match chunk {
                Some(Ok(ViewChunk::Content(content))) => break content,
                Some(Ok(ViewChunk::Swap { .. })) => continue,
                Some(Err(error)) => return Err(error),
                None => break Default::default(),
            }
        };
        let id = self.id;
        if self.first {
            self.first = false;
            self.writer
                .emit(|parts| {
                    parts.push_str_unescaped(&format!("<!--tc:{id}-->"));
                    parts.push_view(content);
                    parts.push_str_unescaped(&format!("<!--/tc:{id}-->"));
                })
                .await;
        } else {
            yield_(Ok(ViewChunk::Swap { id, view: content })).await;
        }
        Ok(())
    }
}
