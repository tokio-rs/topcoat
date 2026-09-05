//! Hoisted parts: content a body pushes while it runs, emitted ahead of the
//! content the enclosing view resolves next.
//!
//! A body sometimes produces markup that belongs at the start of its
//! content rather than at the point the body reached: a marker declaring
//! state, say, that has to come before everything reading it. [`hoist`]
//! takes such a part while a body runs, and the [`HoistView`] around the
//! body prepends everything hoisted to the next content it resolves: its
//! first content, or the replacement of a later swap.
//!
//! The collecting view travels through a thread local installed for exactly
//! the duration of each poll, the way an identity does, so views that
//! interleave on one task never collect each other's parts, and a part goes
//! to the innermost view collecting when it is hoisted.

use std::{
    cell::Cell,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use pin_project_lite::pin_project;
use topcoat_core::error::Result;

use crate::{PartsWriter, View, ViewBuffer, ViewBufferScope, ViewFirst, ViewHandle, ViewSwap};

/// A hoisted part, pushed through the writer of the content it lands in.
type HoistedPart = Box<dyn FnOnce(&mut PartsWriter<'_>) + Send>;

thread_local! {
    /// The parts hoisted so far into the view polling on the current
    /// thread, if one is collecting.
    static CURRENT: Cell<Option<Vec<HoistedPart>>> = const { Cell::new(None) };
}

/// Hoists a part into the content of the enclosing view.
///
/// `build` pushes the part through the writer of the node position it lands
/// in, so a comment marker goes through
/// [`push_comment`](PartsWriter::push_comment) and text through
/// [`push_str`](PartsWriter::push_str). The part renders ahead of the next
/// content the enclosing [`HoistView`] resolves, in the order the parts were
/// hoisted.
///
/// # Panics
///
/// Panics if no view is collecting hoisted parts, which is the case outside
/// a page, layout, component, or shard body, and inside work those bodies
/// spawn onto another task.
#[track_caller]
pub fn hoist(build: impl FnOnce(&mut PartsWriter<'_>) + Send + 'static) {
    let mut collected = CURRENT.take();
    let Some(parts) = &mut collected else {
        panic!(
            "no view is collecting hoisted parts: `hoist` must be called while a page, layout, \
             component, or shard body runs"
        );
    };
    parts.push(Box::new(build));
    CURRENT.set(collected);
}

/// Makes `slot` the collection receiving hoisted parts for exactly the
/// duration of a synchronous region.
///
/// Creating the guard moves the collection into the thread local, and
/// dropping it moves the collection back into `slot`, also when the region
/// panics. A collection installed before is restored at the same time.
#[must_use = "the collection is uninstalled when the guard drops"]
struct HoistGuard<'a> {
    slot: &'a mut Vec<HoistedPart>,
    prev: Option<Vec<HoistedPart>>,
    /// The guard restores a thread local, so it must stay on the thread it
    /// was created on.
    _not_send: PhantomData<*const ()>,
}

impl<'a> HoistGuard<'a> {
    fn install(slot: &'a mut Vec<HoistedPart>) -> Self {
        let prev = CURRENT.replace(Some(std::mem::take(slot)));
        Self {
            slot,
            prev,
            _not_send: PhantomData,
        }
    }
}

impl Drop for HoistGuard<'_> {
    fn drop(&mut self) {
        *self.slot = CURRENT.replace(self.prev.take()).unwrap_or_default();
    }
}

pin_project! {
    /// Collects the parts hoisted while a view polls and prepends them to
    /// the content it resolves.
    ///
    /// A body wrapped in one, together with the view it returns, may call
    /// [`hoist`] at any point. Parts hoisted before the first content
    /// resolves render ahead of that content; parts hoisted later render
    /// ahead of the next swap's replacement. Each poll installs the
    /// collection for exactly its duration, so a nested `HoistView` polled
    /// inside collects its own parts and hands the outer one back
    /// afterwards.
    pub struct HoistView<V> {
        #[pin]
        view: V,
        parts: Vec<HoistedPart>,
    }
}

impl<V: View> HoistView<V> {
    /// Wraps `view` to collect the parts hoisted while it polls.
    pub fn new(view: V) -> Self {
        Self {
            view,
            parts: Vec::new(),
        }
    }
}

impl<V: View> View for HoistView<V> {
    fn poll_first(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<ViewFirst>> {
        let this = self.project();
        let poll = {
            let _guard = HoistGuard::install(this.parts);
            this.view.poll_first(cx)
        };
        match poll {
            Poll::Ready(Ok(ViewFirst { content, live })) => Poll::Ready(Ok(ViewFirst {
                content: prepend(this.parts, content),
                live,
            })),
            poll => poll,
        }
    }

    fn poll_swap(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Option<ViewSwap>>> {
        let this = self.project();
        let poll = {
            let _guard = HoistGuard::install(this.parts);
            this.view.poll_swap(cx)
        };
        match poll {
            Poll::Ready(Ok(Some(ViewSwap {
                region,
                replacement,
            }))) => Poll::Ready(Ok(Some(ViewSwap {
                region,
                replacement: prepend(this.parts, replacement),
            }))),
            poll => poll,
        }
    }
}

/// Splices the parts collected so far ahead of `content`, leaving the
/// collection empty.
///
/// Content with nothing hoisted passes through untouched, so a view that
/// never hoists costs no block in the buffer. Inside a running build the
/// block joins that build's buffer; outside one, as when a swap resolves
/// after the first content was sealed, the block is self-contained.
fn prepend(parts: &mut Vec<HoistedPart>, content: ViewHandle) -> ViewHandle {
    if parts.is_empty() {
        return content;
    }
    let parts = std::mem::take(parts);
    let build = |writer: &mut PartsWriter<'_>| {
        for part in parts {
            part(writer);
        }
        writer.push_view_handle(content);
    };
    if ViewBufferScope::is_active() {
        ViewBufferScope::with(|buffer| buffer.block(build))
    } else {
        let mut buffer = ViewBuffer::new();
        buffer.block(build).seal(buffer)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        panic::{AssertUnwindSafe, catch_unwind},
        pin::pin,
        task::Waker,
    };

    use topcoat_core::context::Cx;

    use super::*;
    use crate::{RegionId, internal::ScopeView};

    /// Hoists a comment carrying `text`.
    fn hoist_comment(text: &'static str) {
        hoist(move |writer| {
            writer.push_comment(|comment| {
                comment.push_static_str(text);
            });
        });
    }

    /// A view that hoists on every poll and resolves static content, then
    /// one swap.
    ///
    /// The first poll of each method is pending, so a test can interleave
    /// two of them across a yield point; the second resolves.
    struct Probe {
        name: &'static str,
        polled_first: bool,
        polled_swap: bool,
    }

    impl Probe {
        fn new(name: &'static str) -> Self {
            Self {
                name,
                polled_first: false,
                polled_swap: false,
            }
        }
    }

    impl View for Probe {
        fn poll_first(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<ViewFirst>> {
            hoist_comment(self.name);
            if std::mem::replace(&mut self.polled_first, true) {
                Poll::Ready(Ok(ViewFirst {
                    content: ViewBufferScope::with(|buffer| {
                        buffer.block(|writer| {
                            writer.push_static_str("content");
                        })
                    }),
                    live: true,
                }))
            } else {
                Poll::Pending
            }
        }

        fn poll_swap(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<Option<ViewSwap>>> {
            if std::mem::replace(&mut self.polled_swap, true) {
                return Poll::Ready(Ok(None));
            }
            hoist_comment("swap");
            // A swap resolves after the first content was sealed, with no
            // build running, so its replacement is self-contained.
            Poll::Ready(Ok(Some(ViewSwap {
                region: RegionId::next(),
                replacement: ViewBuffer::build(|writer| {
                    writer.push_static_str("replacement");
                }),
            })))
        }
    }

    /// Drives `view` to its first content and renders it.
    fn render_first(view: impl View) -> String {
        let mut view = pin!(ScopeView::new(view));
        let mut cx = Context::from_waker(Waker::noop());
        loop {
            if let Poll::Ready(first) = view.as_mut().poll_first(&mut cx) {
                return first.unwrap().content.render(&Cx::default());
            }
        }
    }

    #[test]
    fn hoisting_outside_a_collecting_view_panics() {
        let panic = catch_unwind(|| hoist_comment("x")).unwrap_err();
        let message = panic.downcast::<&str>().expect("panics with a message");
        assert!(message.contains("no view is collecting hoisted parts"));
    }

    #[test]
    fn hoisted_parts_render_ahead_of_the_first_content() {
        let html = render_first(HoistView::new(Probe::new("a")));
        // The probe hoists once per poll and resolves on its second poll.
        assert_eq!(html, "<!--a--><!--a-->content");
    }

    #[test]
    fn hoisted_parts_render_ahead_of_the_next_swap() {
        let mut view = pin!(ScopeView::self_contained(|| HoistView::new(Probe::new(
            "a"
        ))));
        let mut cx = Context::from_waker(Waker::noop());
        while view.as_mut().poll_first(&mut cx).is_pending() {}

        let Poll::Ready(Ok(Some(swap))) = view.as_mut().poll_swap(&mut cx) else {
            panic!("expected a swap");
        };
        assert_eq!(
            swap.replacement.render(&Cx::default()),
            "<!--swap-->replacement"
        );
        assert!(matches!(
            view.as_mut().poll_swap(&mut cx),
            Poll::Ready(Ok(None))
        ));
    }

    #[test]
    fn a_view_that_hoists_nothing_passes_its_content_through() {
        struct Plain;

        impl View for Plain {
            fn poll_first(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<ViewFirst>> {
                Poll::Ready(Ok(ViewFirst {
                    content: ViewHandle::empty(),
                    live: false,
                }))
            }

            fn poll_swap(
                self: Pin<&mut Self>,
                _cx: &mut Context<'_>,
            ) -> Poll<Result<Option<ViewSwap>>> {
                Poll::Ready(Ok(None))
            }
        }

        assert_eq!(render_first(HoistView::new(Plain)), "");
    }

    #[test]
    fn interleaved_siblings_each_collect_their_own_parts() {
        let mut first = pin!(ScopeView::self_contained(|| HoistView::new(Probe::new(
            "a"
        ))));
        let mut second = pin!(ScopeView::self_contained(|| HoistView::new(Probe::new(
            "b"
        ))));
        let mut cx = Context::from_waker(Waker::noop());

        // Interleave the two views across their pending polls.
        assert!(first.as_mut().poll_first(&mut cx).is_pending());
        assert!(second.as_mut().poll_first(&mut cx).is_pending());
        let Poll::Ready(Ok(first)) = first.as_mut().poll_first(&mut cx) else {
            panic!("expected content");
        };
        let Poll::Ready(Ok(second)) = second.as_mut().poll_first(&mut cx) else {
            panic!("expected content");
        };

        assert_eq!(
            first.content.render(&Cx::default()),
            "<!--a--><!--a-->content"
        );
        assert_eq!(
            second.content.render(&Cx::default()),
            "<!--b--><!--b-->content"
        );
    }

    #[test]
    fn a_nested_view_collects_its_own_parts_and_restores_the_outer_ones() {
        /// Hoists into the outer collection around polling an inner view.
        struct Outer<V> {
            inner: Pin<Box<V>>,
        }

        impl<V: View> View for Outer<V> {
            fn poll_first(
                mut self: Pin<&mut Self>,
                cx: &mut Context<'_>,
            ) -> Poll<Result<ViewFirst>> {
                hoist_comment("outer");
                let inner = std::task::ready!(self.inner.as_mut().poll_first(cx))?;
                hoist_comment("outer again");
                Poll::Ready(Ok(ViewFirst {
                    content: ViewBufferScope::with(|buffer| {
                        buffer.block(|writer| {
                            writer.push_static_str("[");
                            writer.push_view_handle(inner.content);
                            writer.push_static_str("]");
                        })
                    }),
                    live: false,
                }))
            }

            fn poll_swap(
                self: Pin<&mut Self>,
                _cx: &mut Context<'_>,
            ) -> Poll<Result<Option<ViewSwap>>> {
                Poll::Ready(Ok(None))
            }
        }

        let html = render_first(HoistView::new(Outer {
            inner: Box::pin(HoistView::new(Probe::new("inner"))),
        }));
        // The outer view hoists on both of its polls and the inner view on
        // both of its own; each lands in its own content.
        assert_eq!(
            html,
            "<!--outer--><!--outer--><!--outer again-->[<!--inner--><!--inner-->content]"
        );
    }

    #[test]
    fn the_collection_is_restored_when_a_poll_panics() {
        struct Boom;

        impl View for Boom {
            fn poll_first(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<ViewFirst>> {
                hoist_comment("lost");
                panic!("boom")
            }

            fn poll_swap(
                self: Pin<&mut Self>,
                _cx: &mut Context<'_>,
            ) -> Poll<Result<Option<ViewSwap>>> {
                panic!("boom")
            }
        }

        let mut view = pin!(HoistView::new(Boom));
        let result = catch_unwind(AssertUnwindSafe(|| {
            let mut cx = Context::from_waker(Waker::noop());
            let _ = view.as_mut().poll_first(&mut cx);
        }));
        assert!(result.is_err());
        // Nothing is collecting any more, so hoisting panics again rather
        // than landing in the abandoned view's collection.
        assert!(catch_unwind(|| hoist_comment("x")).is_err());
        assert_eq!(view.parts.len(), 1);
    }
}
