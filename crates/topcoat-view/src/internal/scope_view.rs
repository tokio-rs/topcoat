use std::{
    pin::Pin,
    task::{Context, Poll},
};

use pin_project_lite::pin_project;
use topcoat_core::error::Result;

use crate::{
    Step, View,
    buffer::{ViewBuffer, ViewBufferScope},
};

pin_project! {
    /// A view polled inside a [`ViewBufferScope`], owning the buffer of the
    /// build when it is the outermost view.
    ///
    /// Every `view!` invocation wraps its template in this type. At its
    /// first poll it decides who owns the build: with a scope already
    /// active, an enclosing view owns the buffer and the template polls
    /// through, appending to that buffer. Otherwise this is the outermost
    /// view: it creates the buffer, installs it around every poll, and seals
    /// it into its first content, which then renders anywhere. An `emit!`
    /// invocation always owns a buffer of its own, so the content it emits
    /// is self-contained regardless of the region it runs in.
    pub struct ScopeView<V> {
        #[pin]
        view: V,
        // The buffer between polls while this view owns the build; `None`
        // once it was sealed into the content, or when an enclosing view
        // owns the build.
        buffer: Option<Box<ViewBuffer>>,
        role: Role,
    }
}

/// Who owns the build a [`ScopeView`] takes part in.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
    /// Decided at the first poll.
    Undecided,
    /// This view owns the buffer, installed around each poll.
    Owner,
    /// This view sealed its buffer into its content.
    Sealed,
    /// An enclosing view owns the buffer.
    Nested,
}

impl<V> ScopeView<V> {
    /// Wraps a view that owns the build when it is the outermost view.
    #[must_use]
    pub fn new(view: V) -> Self {
        Self {
            view,
            buffer: None,
            role: Role::Undecided,
        }
    }

    /// Wraps a view that always owns a buffer of its own, so its content is
    /// self-contained even inside an enclosing build.
    #[must_use]
    pub fn self_contained(view: V) -> Self {
        Self {
            view,
            buffer: Some(Box::new(ViewBuffer::new())),
            role: Role::Owner,
        }
    }
}

impl<V> View for ScopeView<V>
where
    V: View,
{
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Step>> {
        let this = self.project();
        if *this.role == Role::Undecided {
            *this.role = if ViewBufferScope::is_active() {
                Role::Nested
            } else {
                *this.buffer = Some(Box::new(ViewBuffer::new()));
                Role::Owner
            };
        }
        if *this.role != Role::Owner {
            return this.view.poll(cx);
        }
        let step = {
            let _scope = ViewBufferScope::install(this.buffer);
            this.view.poll(cx)
        };
        match step {
            Poll::Ready(Ok(Step::Content { content, live })) => {
                let buffer = this.buffer.take().expect("the owner holds its buffer");
                *this.role = Role::Sealed;
                Poll::Ready(Ok(Step::Content {
                    content: content.seal(*buffer),
                    live,
                }))
            }
            step => step,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{pin::pin, task::Waker};

    use topcoat_core::context::Cx;

    use super::*;
    use crate::{PartsWriter, ViewExt, ViewHandle, buffer::ViewBufferScope};

    /// A view building one block from the parts `f` pushes, in the scope it
    /// is polled in.
    struct Block<F>(Option<F>);

    impl<F> View for Block<F>
    where
        F: FnOnce(&mut PartsWriter<'_>) + Send + Unpin,
    {
        fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<Step>> {
            let f = self.get_mut().0.take().expect("polled once");
            Poll::Ready(Ok(Step::Content {
                content: ViewBufferScope::block(f),
                live: false,
            }))
        }
    }

    /// Resolves `view` to its first content with a single poll.
    fn first(view: impl View) -> ViewHandle {
        let mut view = pin!(view);
        let mut cx = Context::from_waker(Waker::noop());
        match view.as_mut().poll(&mut cx) {
            Poll::Ready(Ok(Step::Content { content, .. })) => content,
            Poll::Ready(Ok(_)) => panic!("the view resolves to content"),
            Poll::Ready(Err(error)) => panic!("{error}"),
            Poll::Pending => panic!("the view is ready at once"),
        }
    }

    #[test]
    fn the_outermost_view_owns_the_buffer_and_seals_its_content() {
        let view = ScopeView::new(Block(Some(|parts: &mut PartsWriter<'_>| {
            parts.push_str("a < b");
        })));
        let content = first(view);
        assert_eq!(content.render(&Cx::default()), "a &lt; b");
    }

    #[test]
    fn a_nested_view_builds_into_the_enclosing_buffer() {
        let mut slot = Some(Box::new(ViewBuffer::new()));
        let content = {
            let _scope = ViewBufferScope::install(&mut slot);
            first(ScopeView::new(Block(Some(
                |parts: &mut PartsWriter<'_>| {
                    parts.push_str("nested");
                },
            ))))
        };
        let buffer = slot.expect("the buffer was swapped back");
        assert_eq!(content.seal(*buffer).render(&Cx::default()), "nested");
    }

    #[test]
    fn a_self_contained_view_owns_a_buffer_inside_an_enclosing_build() {
        let mut slot = Some(Box::new(ViewBuffer::new()));
        let content = {
            let _scope = ViewBufferScope::install(&mut slot);
            first(ScopeView::self_contained(Block(Some(
                |parts: &mut PartsWriter<'_>| {
                    parts.push_str("own");
                },
            ))))
        };
        // The content renders without the enclosing buffer.
        drop(slot);
        assert_eq!(content.render(&Cx::default()), "own");
    }

    #[test]
    fn the_owner_resolves_through_the_view_combinators() {
        let view = ScopeView::new(Block(Some(|parts: &mut PartsWriter<'_>| {
            parts.push_str("x");
        })));
        let mut single = pin!(view.single());
        let mut cx = Context::from_waker(Waker::noop());
        let Poll::Ready(Ok(content)) = single.as_mut().poll(&mut cx) else {
            panic!("the view resolves at once");
        };
        assert_eq!(content.render(&Cx::default()), "x");
    }
}
