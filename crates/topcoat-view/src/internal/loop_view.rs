use std::{
    pin::Pin,
    task::{Context, Poll},
};

use topcoat_core::error::Result;

use crate::{View, ViewBufferScope, ViewFirst, ViewSwap};

pub struct LoopView<V> {
    bodies: Vec<Body<V>>,
    last_swap_index: usize,
}

struct Body<V> {
    view: V,
    ready: Option<ViewFirst>,
    done: bool,
}

impl<V> LoopView<V>
where
    V: View + Unpin,
{
    #[must_use]
    pub fn new(bodies: impl IntoIterator<Item = V>) -> Self {
        Self {
            bodies: bodies
                .into_iter()
                .map(|view| Body {
                    view,
                    ready: None,
                    done: false,
                })
                .collect(),
            last_swap_index: 0,
        }
    }
}

impl<V> View for LoopView<V>
where
    V: View + Unpin,
{
    fn poll_first(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<ViewFirst>> {
        let mut all_ready = true;
        for body in &mut self.bodies {
            if body.ready.is_some() {
                continue;
            }

            match Pin::new(&mut body.view).poll_first(cx) {
                Poll::Pending => all_ready = false,
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Ready(Ok(first)) => {
                    body.ready = Some(first);
                }
            }
        }

        if all_ready {
            let mut live = false;
            let handle = ViewBufferScope::with(|buffer| {
                buffer.block(|parts| {
                    for body in &mut self.bodies {
                        let first = body.ready.take().expect("all loop bodies are ready");
                        parts.push_view_handle(first.content);
                        live |= first.live;
                    }
                })
            });
            Poll::Ready(Ok(ViewFirst {
                content: handle,
                live,
            }))
        } else {
            Poll::Pending
        }
    }

    fn poll_swap(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Option<ViewSwap>>> {
        for i in self.last_swap_index + 1..self.bodies.len() {
            let body = &mut self.bodies[i];
            if body.done {
                continue;
            }
            match Pin::new(&mut body.view).poll_swap(cx) {
                Poll::Pending => continue,
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Ready(Ok(None)) => body.done = true,
                Poll::Ready(Ok(Some(swap))) => {
                    self.last_swap_index = i;
                    return Poll::Ready(Ok(Some(swap)));
                }
            }
        }

        // TODO: not ideal, should loop around.

        self.last_swap_index = 0;
        Poll::Pending
    }
}
