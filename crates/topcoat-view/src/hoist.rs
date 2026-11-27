//! Hoisted parts render before the enclosing view's next content.
//!
//! Call [`hoist`] while rendering to collect a part. The enclosing
//! [`HoistView`] places collected parts before its initial content or
//! next replacement, in the order they were collected.
//!
//! Each part belongs to the innermost active collection. Collections are
//! active only during their view's polls, so interleaved views do not
//! collect each other's parts.

use std::{
    fmt,
    hash::Hash,
    mem,
    pin::Pin,
    task::{Context, Poll},
};

use pin_project_lite::pin_project;
use siphasher::sip128::{Hasher128, SipHasher13};
use topcoat_core::error::Result;

use crate::{PartsWriter, View, ViewFirst, ViewHandle, ViewScope, ViewSwap};

/// A hoisted part, pushed through the writer of the content it lands in.
type HoistedPart = Box<dyn FnOnce(&mut PartsWriter<'_>) + Send>;

/// The key of a part hoisted at most once per content.
///
/// Include a type or tag in the key to distinguish different kinds of part
/// that happen to share an id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HoistKey(u128);

impl HoistKey {
    /// Creates the key of the part identified by `key`.
    pub fn new(key: impl Hash) -> Self {
        let mut hasher = SipHasher13::new();
        key.hash(&mut hasher);
        Self(hasher.finish128().as_u128())
    }
}

/// The parts hoisted into one piece of content, with the keys of those
/// hoisted at most once.
#[derive(Default)]
pub(crate) struct Hoisted {
    parts: Vec<HoistedPart>,
    keys: Vec<HoistKey>,
}

impl Hoisted {
    /// Runs `f` on the collection of the view polling on the current thread.
    ///
    /// # Panics
    ///
    /// Panics if no view is collecting hoisted parts.
    #[track_caller]
    fn current(f: impl FnOnce(&mut Self)) {
        let mut scope = ViewScope::reclaim();
        let Some(hoisted) = scope.as_mut().and_then(|scope| scope.hoisted()) else {
            panic!(
                "no view is collecting hoisted parts: `hoist` must be called while a page, \
                 layout, component, or shard body runs"
            );
        };
        f(hoisted);
    }

    /// Adds a part to the collection.
    fn push(&mut self, build: impl FnOnce(&mut PartsWriter<'_>) + Send + 'static) {
        self.parts.push(Box::new(build));
    }

    /// Adds a part to the collection unless one with the same key is in it.
    fn push_once(
        &mut self,
        key: HoistKey,
        build: impl FnOnce(&mut PartsWriter<'_>) + Send + 'static,
    ) {
        if !self.keys.contains(&key) {
            self.keys.push(key);
            self.push(build);
        }
    }

    /// Splices the parts collected so far ahead of `content`, leaving the
    /// collection empty.
    ///
    /// Content with nothing hoisted passes through untouched, so a view that
    /// never hoists costs no block in the buffer.
    fn prepend(&mut self, content: ViewHandle, scope: &mut ViewScope) -> ViewHandle {
        if self.parts.is_empty() {
            return content;
        }
        self.keys.clear();
        let parts = mem::take(&mut self.parts);
        scope.block(|writer| {
            for part in parts {
                part(writer);
            }
            writer.push_view_handle(content);
        })
    }
}

impl fmt::Debug for Hoisted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Hoisted")
            .field("parts", &self.parts.len())
            .field("keys", &self.keys)
            .finish()
    }
}

/// Hoists a part into the content of the enclosing view.
///
/// `build` receives a writer for node content. Use its methods to select
/// the appropriate HTML context. The part renders before the enclosing
/// [`HoistView`]'s next content, in the order parts were hoisted.
///
/// # Panics
///
/// Panics if no view is collecting hoisted parts. Spawned tasks do not
/// inherit the collection.
#[track_caller]
pub fn hoist(build: impl FnOnce(&mut PartsWriter<'_>) + Send + 'static) {
    Hoisted::current(|hoisted| hoisted.push(build));
}

/// Hoists a part into the content of the enclosing view unless a part with
/// the same key was already hoisted into that content.
///
/// Use this when repeated parts have the same meaning. Each replacement
/// starts a fresh collection, so the same key may appear in a later update.
/// `build` runs only for a part that is kept. Otherwise, [`hoist`]'s rules apply.
///
/// # Panics
///
/// Panics if no view is collecting hoisted parts, like [`hoist`].
#[track_caller]
pub fn hoist_once(key: HoistKey, build: impl FnOnce(&mut PartsWriter<'_>) + Send + 'static) {
    Hoisted::current(|hoisted| hoisted.push_once(key, build));
}

pin_project! {
    /// Collects the parts hoisted while a view polls and prepends them to
    /// the content it resolves.
    ///
    /// Calls to [`hoist`] during a poll add parts to this view's collection.
    /// They render before the next initial content or replacement.
    /// A nested `HoistView` collects its own parts while it is being polled.
    pub struct HoistView<V> {
        #[pin]
        view: V,
        hoisted: Hoisted,
    }
}

impl<V: View> HoistView<V> {
    /// Wraps `view` to collect the parts hoisted while it polls.
    pub fn new(view: V) -> Self {
        Self {
            view,
            hoisted: Hoisted::default(),
        }
    }
}

impl<V: View> View for HoistView<V> {
    fn poll_first(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        scope: &mut ViewScope,
    ) -> Poll<Result<ViewFirst>> {
        let this = self.project();
        let poll = scope.collecting(this.hoisted, |scope| this.view.poll_first(cx, scope));
        match poll {
            Poll::Ready(Ok(ViewFirst { content, live })) => Poll::Ready(Ok(ViewFirst {
                content: this.hoisted.prepend(content, scope),
                live,
            })),
            poll => poll,
        }
    }

    fn poll_swap(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        scope: &mut ViewScope,
    ) -> Poll<Result<Option<ViewSwap>>> {
        let this = self.project();
        let poll = scope.collecting(this.hoisted, |scope| this.view.poll_swap(cx, scope));
        match poll {
            Poll::Ready(Ok(Some(ViewSwap {
                region,
                replacement,
            }))) => Poll::Ready(Ok(Some(ViewSwap {
                region,
                replacement: this.hoisted.prepend(replacement, scope),
            }))),
            poll => poll,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{panic::catch_unwind, pin::pin, task::Waker};

    use topcoat_core::{
        context::Cx,
        identity::{Identity, SiteKey},
    };

    use super::*;
    use crate::{RegionId, ViewBuffer};

    /// Hoists a comment carrying `text` the way code in a view's body does,
    /// with `scope` installed.
    fn hoist_comment(scope: &mut ViewScope, text: &'static str) {
        let _installed = scope.install();
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
        fn poll_first(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            scope: &mut ViewScope,
        ) -> Poll<Result<ViewFirst>> {
            hoist_comment(scope, self.name);
            if std::mem::replace(&mut self.polled_first, true) {
                Poll::Ready(Ok(ViewFirst {
                    content: scope.block(|writer| {
                        writer.push_static_str("content");
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
            scope: &mut ViewScope,
        ) -> Poll<Result<Option<ViewSwap>>> {
            if std::mem::replace(&mut self.polled_swap, true) {
                return Poll::Ready(Ok(None));
            }
            hoist_comment(scope, "swap");
            // A swap resolves after the first content was sealed, with no
            // build running, so its replacement is self-contained.
            Poll::Ready(Ok(Some(ViewSwap {
                region: RegionId::new(Identity::ROOT, SiteKey::new(file!(), line!(), column!(), 0)),
                replacement: ViewBuffer::build(|writer| {
                    writer.push_static_str("replacement");
                }),
            })))
        }
    }

    /// Drives `view` to its first content and renders it.
    fn render_first(view: impl View) -> String {
        let mut view = pin!(view);
        let mut cx = Context::from_waker(Waker::noop());
        let mut scope = ViewScope::new();
        loop {
            if let Poll::Ready(first) = view.as_mut().poll_first(&mut cx, &mut scope) {
                return scope.seal(first.unwrap().content).render(&Cx::new());
            }
        }
    }

    #[test]
    fn hoisting_outside_a_collecting_view_panics() {
        let panic = catch_unwind(|| hoist_comment(&mut ViewScope::new(), "x")).unwrap_err();
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
        let mut view = pin!(HoistView::new(Probe::new("a")));
        let mut cx = Context::from_waker(Waker::noop());
        let mut scope = ViewScope::new();
        while view.as_mut().poll_first(&mut cx, &mut scope).is_pending() {}

        let Poll::Ready(Ok(Some(swap))) = view.as_mut().poll_swap(&mut cx, &mut scope) else {
            panic!("expected a swap");
        };
        assert_eq!(
            scope.seal(swap.replacement).render(&Cx::new()),
            "<!--swap-->replacement"
        );
        assert!(matches!(
            view.as_mut().poll_swap(&mut cx, &mut scope),
            Poll::Ready(Ok(None))
        ));
    }

    #[test]
    fn a_view_that_hoists_nothing_passes_its_content_through() {
        struct Plain;

        impl View for Plain {
            fn poll_first(
                self: Pin<&mut Self>,
                _cx: &mut Context<'_>,
                _scope: &mut ViewScope,
            ) -> Poll<Result<ViewFirst>> {
                Poll::Ready(Ok(ViewFirst {
                    content: ViewHandle::empty(),
                    live: false,
                }))
            }

            fn poll_swap(
                self: Pin<&mut Self>,
                _cx: &mut Context<'_>,
                _scope: &mut ViewScope,
            ) -> Poll<Result<Option<ViewSwap>>> {
                Poll::Ready(Ok(None))
            }
        }

        assert_eq!(render_first(HoistView::new(Plain)), "");
    }

    #[test]
    fn interleaved_siblings_each_collect_their_own_parts() {
        let mut first = pin!(HoistView::new(Probe::new("a")));
        let mut second = pin!(HoistView::new(Probe::new("b")));
        let mut cx = Context::from_waker(Waker::noop());
        let mut first_scope = ViewScope::new();
        let mut second_scope = ViewScope::new();

        // Interleave the two views across their pending polls.
        assert!(
            first
                .as_mut()
                .poll_first(&mut cx, &mut first_scope)
                .is_pending()
        );
        assert!(
            second
                .as_mut()
                .poll_first(&mut cx, &mut second_scope)
                .is_pending()
        );
        let Poll::Ready(Ok(first)) = first.as_mut().poll_first(&mut cx, &mut first_scope) else {
            panic!("expected content");
        };
        let Poll::Ready(Ok(second)) = second.as_mut().poll_first(&mut cx, &mut second_scope) else {
            panic!("expected content");
        };

        assert_eq!(
            first_scope.seal(first.content).render(&Cx::new()),
            "<!--a--><!--a-->content"
        );
        assert_eq!(
            second_scope.seal(second.content).render(&Cx::new()),
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
                scope: &mut ViewScope,
            ) -> Poll<Result<ViewFirst>> {
                hoist_comment(scope, "outer");
                let inner = std::task::ready!(self.inner.as_mut().poll_first(cx, scope))?;
                hoist_comment(scope, "outer again");
                Poll::Ready(Ok(ViewFirst {
                    content: scope.block(|writer| {
                        writer.push_static_str("[");
                        writer.push_view_handle(inner.content);
                        writer.push_static_str("]");
                    }),
                    live: false,
                }))
            }

            fn poll_swap(
                self: Pin<&mut Self>,
                _cx: &mut Context<'_>,
                _scope: &mut ViewScope,
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
    fn a_part_hoisted_in_a_nested_build_reaches_the_enclosing_view() {
        /// Polls its view with a scope of its own, as a build nested in the
        /// one it is polled in.
        struct Nested<V> {
            view: Pin<Box<V>>,
            scope: ViewScope,
        }

        impl<V: View> View for Nested<V> {
            fn poll_first(
                mut self: Pin<&mut Self>,
                cx: &mut Context<'_>,
                scope: &mut ViewScope,
            ) -> Poll<Result<ViewFirst>> {
                let this = &mut *self;
                let poll = scope.nested(&mut this.scope, |inner| {
                    this.view.as_mut().poll_first(cx, inner)
                });
                let first = std::task::ready!(poll)?;
                Poll::Ready(Ok(ViewFirst {
                    content: this.scope.seal(first.content),
                    live: first.live,
                }))
            }

            fn poll_swap(
                self: Pin<&mut Self>,
                _cx: &mut Context<'_>,
                _scope: &mut ViewScope,
            ) -> Poll<Result<Option<ViewSwap>>> {
                Poll::Ready(Ok(None))
            }
        }

        let html = render_first(HoistView::new(Nested {
            view: Box::pin(Probe::new("a")),
            scope: ViewScope::new(),
        }));
        assert_eq!(html, "<!--a--><!--a-->content");
    }

    /// A view that hoists the comments `a` and `b` once each under their
    /// own keys, twice over, on every poll, then resolves static content
    /// and one swap.
    struct OnceProbe {
        resolved_first: bool,
        resolved_swap: bool,
    }

    impl OnceProbe {
        fn hoist_twice(scope: &mut ViewScope) {
            let _installed = scope.install();
            for text in ["a", "b", "a", "b"] {
                hoist_once(HoistKey::new(("comment", text)), move |writer| {
                    writer.push_comment(|comment| {
                        comment.push_static_str(text);
                    });
                });
            }
        }
    }

    impl View for OnceProbe {
        fn poll_first(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            scope: &mut ViewScope,
        ) -> Poll<Result<ViewFirst>> {
            Self::hoist_twice(scope);
            if std::mem::replace(&mut self.resolved_first, true) {
                Poll::Ready(Ok(ViewFirst {
                    content: scope.block(|writer| {
                        writer.push_static_str("content");
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
            scope: &mut ViewScope,
        ) -> Poll<Result<Option<ViewSwap>>> {
            if std::mem::replace(&mut self.resolved_swap, true) {
                return Poll::Ready(Ok(None));
            }
            Self::hoist_twice(scope);
            Poll::Ready(Ok(Some(ViewSwap {
                region: RegionId::new(Identity::ROOT, SiteKey::new(file!(), line!(), column!(), 0)),
                replacement: ViewBuffer::build(|writer| {
                    writer.push_static_str("replacement");
                }),
            })))
        }
    }

    #[test]
    fn a_keyed_part_renders_once_per_content() {
        let mut view = pin!(HoistView::new(OnceProbe {
            resolved_first: false,
            resolved_swap: false,
        }));
        let mut cx = Context::from_waker(Waker::noop());
        let mut scope = ViewScope::new();

        // Two polls, each hoisting both keys twice: one part per key.
        let first = loop {
            if let Poll::Ready(first) = view.as_mut().poll_first(&mut cx, &mut scope) {
                break first.unwrap();
            }
        };
        assert_eq!(
            scope.seal(first.content).render(&Cx::new()),
            "<!--a--><!--b-->content"
        );

        // The swap's replacement is fresh content, so the keys hoist again.
        let Poll::Ready(Ok(Some(swap))) = view.as_mut().poll_swap(&mut cx, &mut scope) else {
            panic!("expected a swap");
        };
        assert_eq!(
            scope.seal(swap.replacement).render(&Cx::new()),
            "<!--a--><!--b-->replacement"
        );
    }

    #[test]
    fn keys_hash_by_value() {
        assert_eq!(HoistKey::new(("dep", 1_u8)), HoistKey::new(("dep", 1_u8)));
        assert_ne!(HoistKey::new(("dep", 1_u8)), HoistKey::new(("dep", 2_u8)));
        assert_ne!(HoistKey::new(("dep", 1_u8)), HoistKey::new(("other", 1_u8)));
    }
}
