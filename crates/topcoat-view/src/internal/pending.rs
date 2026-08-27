use std::{
    pin::Pin,
    task::{Context, Poll},
};

use topcoat_core::error::Result;

use crate::{Step, View, buffer::ViewSlot};

/// What a template has left to drive after its block was built: the views
/// at its reserved node positions.
///
/// A template collects one pending per node position, in the shape the
/// position's place in the template dictates: an `Option` for a position
/// rendered at most once, a `Vec` for one inside a `for` body, and a tuple
/// over all positions. Every pending is `Unpin`, so the collection is
/// driven through plain mutable access; a view that is not `Unpin` boxes
/// itself when it becomes a pending.
pub trait Pending: Unpin {
    /// Polls every view still waiting toward its first content, filling its
    /// slot; ready once all have resolved, or with the first error.
    fn poll_fill(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>>;

    /// Whether any view may still update.
    fn is_live(&self) -> bool;

    /// Polls the views still updating for the next swap, or
    /// [`Step::Done`] once none may update.
    fn poll_swap(&mut self, cx: &mut Context<'_>) -> Poll<Result<Step>>;
}

/// A position filled by a value: nothing is left to drive.
impl Pending for () {
    #[inline]
    fn poll_fill(&mut self, _cx: &mut Context<'_>) -> Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }

    #[inline]
    fn is_live(&self) -> bool {
        false
    }

    #[inline]
    fn poll_swap(&mut self, _cx: &mut Context<'_>) -> Poll<Result<Step>> {
        Poll::Ready(Ok(Step::Done))
    }
}

/// A view driven into the slot reserved for it.
pub struct Slotted<V> {
    /// The view; `None` once it has no further updates.
    view: Option<V>,
    /// The slot; `None` once the view's first content filled it.
    slot: Option<ViewSlot>,
}

impl<V> Slotted<V> {
    pub(crate) fn new(view: V, slot: ViewSlot) -> Self {
        Self {
            view: Some(view),
            slot: Some(slot),
        }
    }
}

impl<V> Pending for Slotted<V>
where
    V: View + Unpin,
{
    fn poll_fill(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>> {
        let Some(slot) = self.slot else {
            return Poll::Ready(Ok(()));
        };
        let view = self
            .view
            .as_mut()
            .expect("a slotted view is kept until its content resolved");
        match Pin::new(view).poll(cx) {
            Poll::Ready(Ok(Step::Content { content, live })) => {
                slot.fill(content);
                self.slot = None;
                if !live {
                    self.view = None;
                }
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Ok(Step::Swap { .. } | Step::Done)) => {
                panic!("a view swapped or completed before its first content")
            }
            Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
            Poll::Pending => Poll::Pending,
        }
    }

    fn is_live(&self) -> bool {
        self.view.is_some()
    }

    fn poll_swap(&mut self, cx: &mut Context<'_>) -> Poll<Result<Step>> {
        let Some(view) = self.view.as_mut() else {
            return Poll::Ready(Ok(Step::Done));
        };
        match Pin::new(view).poll(cx) {
            Poll::Ready(Ok(Step::Swap { swap, live })) => {
                if !live {
                    self.view = None;
                }
                Poll::Ready(Ok(Step::Swap { swap, live }))
            }
            Poll::Ready(Ok(Step::Done)) => {
                self.view = None;
                Poll::Ready(Ok(Step::Done))
            }
            Poll::Ready(Ok(Step::Content { .. })) => panic!("a view produced content twice"),
            Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// A position rendered at most once, skipped by an untaken branch.
impl<P: Pending> Pending for Option<P> {
    #[inline]
    fn poll_fill(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>> {
        match self {
            Some(pending) => pending.poll_fill(cx),
            None => Poll::Ready(Ok(())),
        }
    }

    #[inline]
    fn is_live(&self) -> bool {
        self.as_ref().is_some_and(Pending::is_live)
    }

    #[inline]
    fn poll_swap(&mut self, cx: &mut Context<'_>) -> Poll<Result<Step>> {
        match self {
            Some(pending) => pending.poll_swap(cx),
            None => Poll::Ready(Ok(Step::Done)),
        }
    }
}

/// Polls every entry toward its first content, so all of them make
/// progress together; ready once every entry is, or with the first error.
fn poll_fill_all<P: Pending>(entries: &mut [P], cx: &mut Context<'_>) -> Poll<Result<()>> {
    let mut ready = true;
    for entry in entries {
        match entry.poll_fill(cx) {
            Poll::Ready(Ok(())) => {}
            Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
            Poll::Pending => ready = false,
        }
    }
    if ready {
        Poll::Ready(Ok(()))
    } else {
        Poll::Pending
    }
}

/// Polls every live entry for the next swap.
///
/// Returns the first swap found, reporting whether any entry is still
/// live; `Done` once no entry is.
fn poll_any_swap<P: Pending>(entries: &mut [P], cx: &mut Context<'_>) -> Poll<Result<Step>> {
    let mut pending = false;
    for index in 0..entries.len() {
        if !entries[index].is_live() {
            continue;
        }
        match entries[index].poll_swap(cx) {
            Poll::Ready(Ok(Step::Swap { swap, .. })) => {
                let live = entries.iter().any(Pending::is_live);
                return Poll::Ready(Ok(Step::Swap { swap, live }));
            }
            Poll::Ready(Ok(Step::Done)) => {}
            Poll::Ready(Ok(Step::Content { .. })) => panic!("a view produced content twice"),
            Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
            Poll::Pending => pending = true,
        }
    }
    if pending {
        Poll::Pending
    } else {
        Poll::Ready(Ok(Step::Done))
    }
}

/// A position inside a `for` body, rendered once per pass over it; the
/// passes are driven concurrently.
impl<P: Pending> Pending for Vec<P> {
    fn poll_fill(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>> {
        poll_fill_all(self, cx)
    }

    fn is_live(&self) -> bool {
        self.iter().any(Pending::is_live)
    }

    fn poll_swap(&mut self, cx: &mut Context<'_>) -> Poll<Result<Step>> {
        poll_any_swap(self, cx)
    }
}

/// Implements [`Pending`] for tuples: the positions of a template, driven
/// concurrently.
macro_rules! impl_pending_tuple {
    ($($name:ident $index:tt),*) => {
        impl<$($name: Pending),*> Pending for ($($name,)*) {
            fn poll_fill(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>> {
                let mut ready = true;
                $(
                    match self.$index.poll_fill(cx) {
                        Poll::Ready(Ok(())) => {}
                        Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                        Poll::Pending => ready = false,
                    }
                )*
                if ready {
                    Poll::Ready(Ok(()))
                } else {
                    Poll::Pending
                }
            }

            fn is_live(&self) -> bool {
                false $(|| self.$index.is_live())*
            }

            fn poll_swap(&mut self, cx: &mut Context<'_>) -> Poll<Result<Step>> {
                let mut pending = false;
                $(
                    if self.$index.is_live() {
                        match self.$index.poll_swap(cx) {
                            Poll::Ready(Ok(Step::Swap { swap, .. })) => {
                                let live = self.is_live();
                                return Poll::Ready(Ok(Step::Swap { swap, live }));
                            }
                            Poll::Ready(Ok(Step::Done)) => {}
                            Poll::Ready(Ok(Step::Content { .. })) => {
                                panic!("a view produced content twice")
                            }
                            Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                            Poll::Pending => pending = true,
                        }
                    }
                )*
                if pending {
                    Poll::Pending
                } else {
                    Poll::Ready(Ok(Step::Done))
                }
            }
        }
    };
}

impl_pending_tuple!(A 0);
impl_pending_tuple!(A 0, B 1);
impl_pending_tuple!(A 0, B 1, C 2);
impl_pending_tuple!(A 0, B 1, C 2, D 3);
impl_pending_tuple!(A 0, B 1, C 2, D 3, E 4);
impl_pending_tuple!(A 0, B 1, C 2, D 3, E 4, F 5);
impl_pending_tuple!(A 0, B 1, C 2, D 3, E 4, F 5, G 6);
impl_pending_tuple!(A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7);
impl_pending_tuple!(A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8);
impl_pending_tuple!(A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9);
impl_pending_tuple!(A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9, K 10);
impl_pending_tuple!(A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9, K 10, L 11);
impl_pending_tuple!(A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9, K 10, L 11, M 12);
impl_pending_tuple!(A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9, K 10, L 11, M 12, N 13);
impl_pending_tuple!(A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9, K 10, L 11, M 12, N 13, O 14);
impl_pending_tuple!(A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9, K 10, L 11, M 12, N 13, O 14, P 15);
