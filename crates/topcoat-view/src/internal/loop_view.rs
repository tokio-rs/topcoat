use std::{
    pin::Pin,
    task::{Context, Poll},
};

use topcoat_core::{context::Cx, error::Result};

use crate::{
    Swap, View,
    buffer::{ViewBuffer, ViewHandle},
};

/// The iterations of a `for` body as one node value.
///
/// Every iteration is driven toward its content; once all have resolved,
/// the loop's own block splices them in iteration order. After that the
/// iterations' updates merge into one stream of swaps.
///
/// The iterations share one view type, so the expansion boxes each body.
/// The `Unpin` bound this trades on is what lets the loop hold its views in
/// a plain `Vec`.
pub struct LoopView<V> {
    iterations: Vec<Iteration<V>>,
}

struct Iteration<V> {
    /// The iteration's view; `None` once it has no further updates.
    view: Option<V>,
    /// The iteration's content, held until the loop's block splices it.
    content: Option<ViewHandle>,
}

impl<V> LoopView<V>
where
    V: View + Unpin,
{
    #[must_use]
    pub fn new(views: Vec<V>) -> Self {
        Self {
            iterations: views
                .into_iter()
                .map(|view| Iteration {
                    view: Some(view),
                    content: None,
                })
                .collect(),
        }
    }
}

impl<V> View for LoopView<V>
where
    V: View + Unpin,
{
    fn poll_first(
        self: Pin<&mut Self>,
        cx: &Cx,
        task: &mut Context<'_>,
        buf: &mut ViewBuffer,
    ) -> Poll<Result<ViewHandle>> {
        let this = self.get_mut();
        let mut ready = true;
        for iteration in &mut this.iterations {
            if iteration.content.is_some() {
                continue;
            }
            let view = iteration
                .view
                .as_mut()
                .expect("`poll_first` called again after it returned `Ready`");
            match Pin::new(view).poll_first(cx, task, buf) {
                Poll::Ready(Ok(content)) => iteration.content = Some(content),
                Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                Poll::Pending => ready = false,
            }
        }
        if !ready {
            return Poll::Pending;
        }
        let view = buf.block(cx, |b| {
            for iteration in &mut this.iterations {
                let content = iteration
                    .content
                    .take()
                    .expect("every iteration resolved its content");
                b.view(content);
            }
        });
        Poll::Ready(Ok(view))
    }

    fn poll_swap(
        self: Pin<&mut Self>,
        cx: &Cx,
        task: &mut Context<'_>,
        buf: &mut ViewBuffer,
    ) -> Poll<Option<Result<Swap>>> {
        let this = self.get_mut();
        let mut pending = false;
        for iteration in &mut this.iterations {
            let Some(view) = iteration.view.as_mut() else {
                continue;
            };
            match Pin::new(view).poll_swap(cx, task, buf) {
                Poll::Ready(Some(item)) => return Poll::Ready(Some(item)),
                Poll::Ready(None) => iteration.view = None,
                Poll::Pending => pending = true,
            }
        }
        if pending {
            Poll::Pending
        } else {
            Poll::Ready(None)
        }
    }
}
