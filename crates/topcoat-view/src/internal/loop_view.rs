use std::{
    pin::Pin,
    task::{Context, Poll},
};

use topcoat_core::error::Result;

use crate::{
    Step, View,
    buffer::{ViewBufferScope, ViewHandle},
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
    /// Whether the loop's block was built from the iterations' contents.
    built: bool,
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
            built: false,
        }
    }

    /// Polls every iteration still waiting toward its content, and builds
    /// the loop's block once all have resolved.
    fn poll_content(&mut self, cx: &mut Context<'_>) -> Poll<Result<Step>> {
        let mut ready = true;
        for iteration in &mut self.iterations {
            if iteration.content.is_some() {
                continue;
            }
            let view = iteration
                .view
                .as_mut()
                .expect("an iteration keeps its view until its content resolved");
            match Pin::new(view).poll(cx) {
                Poll::Ready(Ok(Step::Content { content, live })) => {
                    iteration.content = Some(content);
                    if !live {
                        iteration.view = None;
                    }
                }
                Poll::Ready(Ok(Step::Swap { .. } | Step::Done)) => {
                    panic!("a view swapped or completed before its first content")
                }
                Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                Poll::Pending => ready = false,
            }
        }
        if !ready {
            return Poll::Pending;
        }
        let content = ViewBufferScope::block(|parts| {
            for iteration in &mut self.iterations {
                let content = iteration
                    .content
                    .take()
                    .expect("every iteration resolved its content");
                parts.push_view_handle(content);
            }
        });
        self.built = true;
        Poll::Ready(Ok(Step::Content {
            content,
            live: self.is_live(),
        }))
    }

    /// Polls the iterations still updating for the next swap.
    fn poll_swap(&mut self, cx: &mut Context<'_>) -> Poll<Result<Step>> {
        let mut pending = false;
        let mut swapped = None;
        for iteration in &mut self.iterations {
            let Some(view) = iteration.view.as_mut() else {
                continue;
            };
            match Pin::new(view).poll(cx) {
                Poll::Ready(Ok(Step::Swap { swap, live })) => {
                    if !live {
                        iteration.view = None;
                    }
                    swapped = Some(swap);
                    break;
                }
                Poll::Ready(Ok(Step::Done)) => iteration.view = None,
                Poll::Ready(Ok(Step::Content { .. })) => panic!("a view produced content twice"),
                Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                Poll::Pending => pending = true,
            }
        }
        match swapped {
            Some(swap) => Poll::Ready(Ok(Step::Swap {
                swap,
                live: self.is_live(),
            })),
            None if pending => Poll::Pending,
            None => Poll::Ready(Ok(Step::Done)),
        }
    }

    /// Whether any iteration may still update.
    fn is_live(&self) -> bool {
        self.iterations
            .iter()
            .any(|iteration| iteration.view.is_some())
    }
}

impl<V> View for LoopView<V>
where
    V: View + Unpin,
{
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Step>> {
        let this = self.get_mut();
        if this.built {
            this.poll_swap(cx)
        } else {
            this.poll_content(cx)
        }
    }
}
