use core::fmt;
use std::sync::Arc;

#[cfg(feature = "http")]
use http::{HeaderMap, StatusCode};
use topcoat_core::context::Cx;

use crate::{
    Formatter, HtmlContext, HtmlWriter,
    arena::{Arena, ArenaId, ArenaScope, InstructionPtr, Renderer},
};

/// A self-contained piece of HTML content.
///
/// A view may contain multiple sibling nodes, but opened tags must be closed
/// so the fragment can be nested safely inside a larger document.
///
/// ```html
/// <!-- Valid: all tags are closed, safe to nest -->
/// <div>Hello</div>
/// <p>World</p>
///
/// <!-- Invalid: unclosed tag would corrupt the parent document -->
/// <div>Hello
/// ```
///
/// The outermost `view!` invocation allocates an instruction arena that the
/// returned view owns; every `view!` nested inside it, such as a component
/// body, appends to that same arena and returns a cheap handle into it. An
/// owned view is a self-contained value: it can be stored, sent across
/// tasks, spliced into another view, and rendered anywhere. A nested handle
/// only lives while its enclosing invocation builds; one that escapes it
/// panics when used.
#[derive(Debug, Default, Clone)]
pub struct View {
    repr: ViewRepr,
}

/// The kinds of view: a static string independent of any arena, a handle
/// into the arena of an enclosing `view!` invocation still building, or an
/// owned view carrying its own arena.
#[derive(Debug, Clone)]
pub(crate) enum ViewRepr {
    /// Trusted static markup rendered verbatim, independent of any arena.
    Static(&'static str),
    /// An instruction block in the still building arena identified by
    /// `arena`, starting at `entry`.
    Scoped {
        arena: ArenaId,
        entry: InstructionPtr,
        /// An estimate of the number of bytes the block writes when
        /// rendered, accumulated while the view was built.
        size_hint: usize,
    },
    /// An instruction block starting at `entry` in an arena the view holds
    /// on to itself.
    Owned {
        arena: Arc<Arena>,
        entry: InstructionPtr,
        /// An estimate of the number of bytes the block writes when
        /// rendered, accumulated while the view was built.
        size_hint: usize,
    },
}

impl Default for ViewRepr {
    #[inline]
    fn default() -> Self {
        Self::Static("")
    }
}

impl View {
    /// Creates the handle for an instruction block built in the arena
    /// identified by `arena`, estimated to write `size_hint` bytes.
    #[inline]
    pub(crate) fn from_scope(arena: ArenaId, entry: InstructionPtr, size_hint: usize) -> Self {
        Self {
            repr: ViewRepr::Scoped {
                arena,
                entry,
                size_hint,
            },
        }
    }

    /// Unwraps the view into its representation.
    #[inline]
    pub(crate) fn repr(self) -> ViewRepr {
        self.repr
    }

    /// Returns an estimate of the number of bytes the view writes when
    /// rendered.
    #[inline]
    pub(crate) fn size_hint(&self) -> usize {
        match &self.repr {
            ViewRepr::Static(body) => body.len(),
            ViewRepr::Scoped { size_hint, .. } | ViewRepr::Owned { size_hint, .. } => *size_hint,
        }
    }

    /// Seals a root build: the view takes ownership of the arena its
    /// instructions were appended to. With no arena, the build was nested
    /// and the view stays a handle into the enclosing invocation's arena.
    ///
    /// A static view carries no instructions, so it passes through and the
    /// arena is dropped.
    ///
    /// # Panics
    ///
    /// Panics if the view's instructions live in a different arena.
    pub(crate) fn seal(self, arena: Option<Arena>) -> Self {
        let Some(arena) = arena else {
            return self;
        };
        match self.repr {
            ViewRepr::Static(_) | ViewRepr::Owned { .. } => self,
            ViewRepr::Scoped {
                arena: id,
                entry,
                size_hint,
            } => {
                assert!(
                    id == arena.id(),
                    "tried to seal a view into an arena it was not built in",
                );
                Self {
                    repr: ViewRepr::Owned {
                        arena: Arc::new(arena),
                        entry,
                        size_hint,
                    },
                }
            }
        }
    }

    /// Returns a `View` that renders to an empty string.
    #[inline]
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Returns `true` if the view is statically known to render no output.
    ///
    /// A view holding an instruction block reports `false` even when the
    /// block happens to write nothing.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        matches!(self.repr, ViewRepr::Static(""))
    }

    /// Creates a view from a `&'static str` without escaping it and without checking for syntax
    /// errors.
    #[inline]
    #[must_use]
    pub const fn unescaped_unchecked(body: &'static str) -> Self {
        Self {
            repr: ViewRepr::Static(body),
        }
    }

    /// Renders the view into an HTML string.
    #[cfg_attr(
        feature = "http",
        doc = "",
        doc = "Status codes and headers declared in the view are discarded;",
        doc = "[`render_response`](Self::render_response) collects them."
    )]
    ///
    /// # Panics
    ///
    /// Panics if the view is a nested handle that escaped the `view!`
    /// invocation it was built in, or if a dynamic attribute key or element
    /// name in the view contains a character that could break out of the
    /// identifier.
    #[track_caller]
    pub fn render(self, cx: &Cx) -> String {
        match self.repr {
            ViewRepr::Static(body) => body.to_owned(),
            ViewRepr::Scoped {
                arena,
                entry,
                size_hint,
            } => {
                let mut html = String::with_capacity(size_hint);
                let mut f = Formatter::new(&mut html);
                Self::execute(arena, entry, cx, &mut f);
                html
            }
            ViewRepr::Owned {
                arena,
                entry,
                size_hint,
            } => {
                let mut html = String::with_capacity(size_hint);
                let mut f = Formatter::new(&mut html);
                Renderer::new(&arena, entry).execute(cx, &mut f);
                html
            }
        }
    }

    /// Renders the view into HTML together with the status code and response
    /// headers declared in it.
    ///
    /// A view declares response metadata by placing an
    /// [`http::StatusCode`](StatusCode), an [`http::HeaderMap`](HeaderMap),
    /// or a single `(HeaderName, HeaderValue)` pair in the node position of
    /// the `view!` macro. Competing declarations resolve by render order:
    /// the first status code rendered wins, and the first part that mentions
    /// a header name provides all of that name's values.
    ///
    /// # Panics
    ///
    /// Panics if the view is a nested handle that escaped the `view!`
    /// invocation it was built in, or if a dynamic attribute key or element
    /// name in the view contains a character that could break out of the
    /// identifier.
    #[cfg(feature = "http")]
    #[must_use]
    #[track_caller]
    pub fn render_response(self, cx: &Cx) -> RenderedResponse {
        match self.repr {
            ViewRepr::Static(body) => RenderedResponse {
                html: body.to_owned(),
                status_code: None,
                headers: HeaderMap::new(),
            },
            ViewRepr::Scoped {
                arena,
                entry,
                size_hint,
            } => {
                let mut html = String::with_capacity(size_hint);
                let mut f = Formatter::new(&mut html);
                Self::execute(arena, entry, cx, &mut f);
                let (status_code, headers) = f.into_recorded();
                RenderedResponse {
                    html,
                    status_code,
                    headers,
                }
            }
            ViewRepr::Owned {
                arena,
                entry,
                size_hint,
            } => {
                let mut html = String::with_capacity(size_hint);
                let mut f = Formatter::new(&mut html);
                Renderer::new(&arena, entry).execute(cx, &mut f);
                let (status_code, headers) = f.into_recorded();
                RenderedResponse {
                    html,
                    status_code,
                    headers,
                }
            }
        }
    }

    /// Executes a nested view handle's instruction block against the arena
    /// of the enclosing `view!` invocation still building it.
    #[track_caller]
    fn execute(arena: ArenaId, entry: InstructionPtr, cx: &Cx, f: &mut Formatter<'_>) {
        ArenaScope::with(|active| {
            assert!(
                active.id() == arena,
                "tried to render a view outside the `view!` invocation it was built in",
            );
            Renderer::new(active, entry).execute(cx, f);
        });
    }
}

/// The output of rendering a [`View`] for an HTTP response.
///
/// Returned by [`View::render_response`]: the rendered HTML alongside the
/// status code and headers the view declared.
#[cfg(feature = "http")]
#[derive(Debug)]
#[non_exhaustive]
pub struct RenderedResponse {
    /// The rendered HTML.
    pub html: String,
    /// The first status code the render encountered, if any.
    pub status_code: Option<StatusCode>,
    /// The collected response headers.
    ///
    /// Each name carries the values of the first render part that mentioned
    /// it.
    pub headers: HeaderMap,
}

/// A boxed view part that writes its output at render time.
///
/// Implement this for values whose output is only known when the view
/// renders, such as resolved asset URLs. The writer passed to
/// [`render`](Self::render) already carries the [`HtmlContext`] of the
/// position the part was pushed into, so everything written through it is
/// escaped or validated for that position.
pub trait DynViewPart: 'static + fmt::Debug + Send + Sync {
    /// Writes this part's output into `w`.
    #[track_caller]
    fn render(&self, cx: &Cx, w: &mut HtmlWriter<'_, '_>);

    /// Returns an estimate of the number of bytes this part will write.
    ///
    /// Used to pre-allocate the output buffer, so aim for a close estimate. A
    /// slight over-estimate is usually preferable to an under-estimate.
    #[inline]
    fn size_hint(&self) -> usize {
        0
    }
}

macro_rules! impl_push_primitive {
    ($method:ident, $ty:ty, $size_hint:expr) => {
        #[doc = concat!("Appends a `", stringify!($ty), "` rendered as text.")]
        ///
        /// Its rendered form contains no character that is significant in any
        /// HTML context, so no escaping applies.
        #[inline]
        pub fn $method(&mut self, value: $ty) -> &mut Self {
            self.size_hint += $size_hint;
            self.arena.$method(value);
            self
        }
    };
}

/// A context-carrying writer over an instruction arena, created per
/// position.
///
/// The `view!` macro creates a `PartsWriter` for each dynamic position it
/// fills and hands it to the matching position trait:
/// [`NodeViewParts`](crate::NodeViewParts),
/// [`AttributeValueViewParts`](crate::AttributeValueViewParts),
/// [`AttributeKeyViewParts`](crate::AttributeKeyViewParts),
/// [`ElementNameViewParts`](crate::ElementNameViewParts), or
/// [`AttributeViewParts`](crate::AttributeViewParts).
///
/// Implementations of those traits make a value renderable by pushing it
/// through the `push_*` methods, which seal the pushed text with the
/// [`HtmlContext`] of the position so rendering escapes or validates it
/// correctly, or by delegating to another implementation of the same
/// position trait. The `push_*_unescaped` methods are the only way to opt
/// out of that protection.
///
/// The writer also accumulates a size hint: an estimate of the number of
/// bytes everything pushed so far will write when rendered. The estimate
/// becomes the built view's size hint, which pre-allocates the output buffer
/// at render time.
pub struct PartsWriter<'a> {
    arena: &'a mut Arena,
    context: HtmlContext,
    size_hint: usize,
}

impl<'a> PartsWriter<'a> {
    /// Creates a writer that seals everything pushed into it with `context`.
    #[inline]
    fn new(arena: &'a mut Arena, context: HtmlContext) -> Self {
        Self {
            arena,
            context,
            size_hint: 0,
        }
    }

    /// Appends one view's instruction block to `arena`, filled by `f`
    /// through a writer in text context.
    ///
    /// Records the entry address, runs `f`, and terminates the block with a
    /// return instruction. Returns the handle to the block, carrying the
    /// writer's accumulated size hint.
    pub(crate) fn block(arena: &mut Arena, f: impl FnOnce(&mut PartsWriter<'_>)) -> View {
        let entry = arena.next_ptr();
        let mut parts = PartsWriter::new(arena, HtmlContext::Text);
        f(&mut parts);
        let size_hint = parts.size_hint();
        arena.push_ret();
        View::from_scope(arena.id(), entry, size_hint)
    }

    /// Returns the accumulated size hint of everything pushed so far.
    #[inline]
    pub(crate) fn size_hint(&self) -> usize {
        self.size_hint
    }

    /// Runs `f` with this writer sealing for a different context, then
    /// restores the current context.
    ///
    /// In-crate compositions that span more than one position use this to
    /// transition between the positions they cover, such as
    /// [`Attribute`](crate::Attribute) moving from a key to a value or
    /// [`push_comment`](Self::push_comment) sealing a comment body.
    #[inline]
    pub(crate) fn in_context<R>(
        &mut self,
        context: HtmlContext,
        f: impl FnOnce(&mut Self) -> R,
    ) -> R {
        let previous = std::mem::replace(&mut self.context, context);
        let result = f(self);
        self.context = previous;
        result
    }

    /// Estimates the bytes `value` writes when rendered in `context`.
    fn str_size_hint(value: &str, context: HtmlContext) -> usize {
        match context {
            HtmlContext::Unescaped => value.len(),
            // Assume some characters escape into multi-byte sequences.
            _ => value.len() + value.len() / 8,
        }
    }

    /// Appends a borrowed string, sealed with this writer's context.
    #[inline]
    pub fn push_str(&mut self, value: &str) -> &mut Self {
        self.size_hint += Self::str_size_hint(value, self.context);
        self.arena.push_str(value, self.context);
        self
    }

    /// Appends a static string, sealed with this writer's context.
    #[inline]
    pub fn push_static_str(&mut self, value: &'static str) -> &mut Self {
        self.size_hint += Self::str_size_hint(value, self.context);
        self.arena.push_static_str(value, self.context);
        self
    }

    /// Appends an owned string, sealed with this writer's context.
    #[inline]
    pub fn push_string(&mut self, value: String) -> &mut Self {
        self.size_hint += Self::str_size_hint(&value, self.context);
        self.arena.push_string(value, self.context);
        self
    }

    /// Appends a borrowed string that renders verbatim, bypassing this
    /// writer's context.
    ///
    /// Use this only for trusted markup. Passing untrusted input defeats the
    /// runtime's escaping and can lead to XSS vulnerabilities.
    #[inline]
    pub fn push_str_unescaped(&mut self, value: &str) -> &mut Self {
        self.size_hint += value.len();
        self.arena.push_str(value, HtmlContext::Unescaped);
        self
    }

    /// Appends a static string that renders verbatim, bypassing this
    /// writer's context.
    ///
    /// Use this only for trusted markup. Passing untrusted input defeats the
    /// runtime's escaping and can lead to XSS vulnerabilities.
    #[inline]
    pub fn push_static_str_unescaped(&mut self, value: &'static str) -> &mut Self {
        self.size_hint += value.len();
        self.arena.push_static_str(value, HtmlContext::Unescaped);
        self
    }

    /// Appends an owned string that renders verbatim, bypassing this
    /// writer's context.
    ///
    /// Use this only for trusted markup. Passing untrusted input defeats the
    /// runtime's escaping and can lead to XSS vulnerabilities.
    #[inline]
    pub fn push_string_unescaped(&mut self, value: String) -> &mut Self {
        self.size_hint += value.len();
        self.arena.push_string(value, HtmlContext::Unescaped);
        self
    }

    /// Appends an HTML comment whose body is built through `build`.
    ///
    /// The `<!-- ` and ` -->` delimiters are written verbatim, while the
    /// writer handed to `build` seals everything pushed into it for the
    /// [`Comment`](HtmlContext::Comment) context. Because that context
    /// escapes `>`, the body can never contain `-->` and terminate the
    /// comment, so a marker can be built from untrusted data with
    /// [`push_str`](Self::push_str) and no separate escaping step.
    ///
    /// # Panics
    ///
    /// Panics if used in a non-text HTML context.
    #[inline]
    pub fn push_comment(&mut self, build: impl FnOnce(&mut PartsWriter<'_>)) -> &mut Self {
        assert!(
            self.context == HtmlContext::Text,
            "tried to push comment in html context {:?}",
            self.context,
        );
        self.push_static_str_unescaped("<!-- ");
        self.in_context(HtmlContext::Comment, build);
        self.push_static_str_unescaped(" -->");
        self
    }

    /// Appends a character, sealed with this writer's context.
    #[inline]
    pub fn push_char(&mut self, value: char) -> &mut Self {
        // One to four UTF-8 bytes, or an escape sequence.
        self.size_hint += 3;
        self.arena.push_char(value, self.context);
        self
    }

    // Each numeric size hint is the midpoint, rounded up, between the
    // shortest and widest output the type can render, including the leading
    // `-` for signed types (`isize`/`usize` assume a 64-bit target). A
    // float's rendered width is unbounded for extreme magnitudes, so the
    // upper end is the shortest round-trip form of a typical value.

    impl_push_primitive!(push_bool, bool, 5);
    impl_push_primitive!(push_i8, i8, 3);
    impl_push_primitive!(push_i16, i16, 4);
    impl_push_primitive!(push_i32, i32, 6);
    impl_push_primitive!(push_i64, i64, 11);
    impl_push_primitive!(push_i128, i128, 21);
    impl_push_primitive!(push_isize, isize, 11);
    impl_push_primitive!(push_u8, u8, 2);
    impl_push_primitive!(push_u16, u16, 3);
    impl_push_primitive!(push_u32, u32, 6);
    impl_push_primitive!(push_u64, u64, 11);
    impl_push_primitive!(push_u128, u128, 20);
    impl_push_primitive!(push_usize, usize, 11);
    impl_push_primitive!(push_f32, f32, 9);
    impl_push_primitive!(push_f64, f64, 13);

    /// Appends a part that writes its output at render time, sealed with
    /// this writer's context.
    #[inline]
    pub fn push_dyn(&mut self, part: Box<dyn DynViewPart>) -> &mut Self {
        self.size_hint += part.size_hint();
        self.arena.push_dyn(part, self.context);
        self
    }

    /// Appends a nested view.
    ///
    /// The view's content was already sealed with the contexts it was built
    /// for; this writer's context does not apply. The view's size hint joins
    /// this writer's, so a view spliced twice counts its output twice.
    ///
    /// # Panics
    ///
    /// Panics if the view was built in a different, still building arena.
    #[inline]
    pub(crate) fn push_view(&mut self, view: View) -> &mut Self {
        self.size_hint += view.size_hint();
        self.arena.push_view(view);
        self
    }

    /// Records a response status code; renders no content.
    #[cfg(feature = "http")]
    #[inline]
    pub fn push_status_code(&mut self, status_code: StatusCode) -> &mut Self {
        self.arena.push_status_code(status_code);
        self
    }

    /// Records response headers; renders no content.
    #[cfg(feature = "http")]
    #[inline]
    pub fn push_headers(&mut self, headers: HeaderMap) -> &mut Self {
        self.arena.push_headers(headers);
        self
    }
}

#[cfg(test)]
mod tests {
    use std::{
        future::Future,
        pin::pin,
        task::{Context, Poll, Waker},
    };

    use super::*;
    use crate::internal::{build, build_sync, reserve};

    /// Runs `f` with a request context inside a fresh view scope.
    fn in_scope<R>(f: impl AsyncFnOnce(&Cx) -> R) -> R {
        block_on(ArenaScope::scope(async { f(&Cx::default()).await })).0
    }

    /// Builds a view outside any scope, so it owns its arena.
    fn owned(f: impl FnOnce(&mut PartsWriter<'_>)) -> View {
        build_sync(f)
    }

    /// Drives `fut` to completion on the current thread.
    ///
    /// The futures under test never wait on external events, so polling in a
    /// tight loop is sufficient.
    fn block_on<F: Future>(fut: F) -> F::Output {
        let mut fut = pin!(fut);
        let mut task = Context::from_waker(Waker::noop());
        loop {
            if let Poll::Ready(output) = fut.as_mut().poll(&mut task) {
                return output;
            }
        }
    }

    /// Builds a view inside a fresh scope through a writer sealed with
    /// `context` and renders it.
    fn render_with(context: HtmlContext, f: impl FnOnce(&mut PartsWriter<'_>)) -> String {
        in_scope(async |cx| build_sync(|parts| parts.in_context(context, f)).render(cx))
    }

    #[test]
    fn static_views_render_without_a_scope() {
        assert_eq!(View::empty().render(&Cx::default()), "");
        let view = View::unescaped_unchecked("<b>raw</b>");
        assert_eq!(view.render(&Cx::default()), "<b>raw</b>");
    }

    #[test]
    fn push_str_seals_the_writer_context() {
        let out = render_with(HtmlContext::Text, |w| {
            w.push_str("<b> & \"q\"");
        });
        assert_eq!(out, "&lt;b&gt; &amp; \"q\"");

        let out = render_with(HtmlContext::AttributeValue, |w| {
            w.push_str("<b> & \"q\"");
        });
        assert_eq!(out, "<b> &amp; &quot;q&quot;");
    }

    #[test]
    fn push_str_unescaped_bypasses_the_context() {
        let out = render_with(HtmlContext::Text, |w| {
            w.push_str_unescaped("<b>raw</b>");
        });
        assert_eq!(out, "<b>raw</b>");
    }

    #[test]
    fn push_char_seals_the_writer_context() {
        let out = render_with(HtmlContext::Text, |w| {
            w.push_char('<');
        });
        assert_eq!(out, "&lt;");
    }

    #[test]
    #[should_panic(expected = "invalid attribute key")]
    fn ident_context_panics_on_forbidden_characters_at_render() {
        render_with(HtmlContext::AttributeKey, |w| {
            w.push_str("on click");
        });
    }

    #[test]
    fn push_primitives_render_as_text() {
        let out = render_with(HtmlContext::Text, |w| {
            w.push_i32(-42).push_str_unescaped(" ");
            w.push_bool(true).push_str_unescaped(" ");
            w.push_f64(1.5).push_str_unescaped(" ");
            w.push_i128(-1 << 100).push_str_unescaped(" ");
            w.push_u128(1 << 100);
        });
        assert_eq!(
            out,
            "-42 true 1.5 -1267650600228229401496703205376 1267650600228229401496703205376"
        );
    }

    #[test]
    fn push_view_splices_nested_views() {
        in_scope(async |cx| {
            let inner = build_sync(|parts| {
                parts.push_str("a < b");
            });

            let outer = build_sync(|parts| {
                parts.push_str_unescaped("<p>");
                parts.push_view(inner);
                parts.push_str_unescaped("</p>");
            });
            assert_eq!(outer.render(cx), "<p>a &lt; b</p>");
        });
    }

    #[test]
    fn document_order_follows_splice_order_not_arena_order() {
        in_scope(async |cx| {
            // Built in reverse: `second` occupies earlier arena addresses.
            let second = build_sync(|parts| {
                parts.push_str("B");
            });
            let first = build_sync(|parts| {
                parts.push_str("A");
            });

            let outer = build_sync(|parts| {
                parts.push_view(first);
                parts.push_view(second);
            });
            assert_eq!(outer.render(cx), "AB");
        });
    }

    #[test]
    fn owned_views_render_without_an_active_build() {
        let view = owned(|parts| {
            parts.push_str("a < b");
        });
        assert_eq!(view.render(&Cx::default()), "a &lt; b");
    }

    #[test]
    fn owned_views_splice_across_memories() {
        let inner = owned(|parts| {
            parts.push_str("a < b");
        });

        let outer = owned(|parts| {
            parts.push_str_unescaped("<p>");
            parts.push_view(inner);
            parts.push_str_unescaped("</p>");
        });
        assert_eq!(outer.render(&Cx::default()), "<p>a &lt; b</p>");
    }

    #[test]
    fn owned_views_splice_into_an_active_build() {
        let inner = owned(|parts| {
            parts.push_str("a < b");
        });

        in_scope(async |cx| {
            let outer = build_sync(|parts| {
                parts.push_str_unescaped("<p>");
                parts.push_view(inner);
                parts.push_str_unescaped("</p>");
            });
            assert_eq!(outer.render(cx), "<p>a &lt; b</p>");
        });
    }

    #[test]
    fn owned_views_fill_a_slot_like_nested_ones() {
        let inner = owned(|parts| {
            parts.push_str("a < b");
        });

        in_scope(async |cx| {
            let (placeholder, slot) = reserve();
            slot.fill(inner);
            assert_eq!(placeholder.render(cx), "a &lt; b");
        });
    }

    #[test]
    fn nested_root_invocations_append_to_the_enclosing_arena() {
        in_scope(async |cx| {
            let inner = build(async {
                Ok(build_sync(|parts| {
                    parts.push_str("x");
                }))
            })
            .await
            .expect("the build is infallible");
            assert!(matches!(inner.repr, ViewRepr::Scoped { .. }));

            let outer = build_sync(|parts| {
                parts.push_view(inner);
            });
            assert_eq!(outer.render(cx), "x");
        });
    }

    #[test]
    fn owned_views_are_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>(_value: &T) {}

        let view = owned(|parts| {
            parts.push_str("x");
        });
        assert_send_sync(&view);
    }

    #[test]
    fn static_views_are_spliced_verbatim() {
        in_scope(async |cx| {
            let outer = build_sync(|parts| {
                parts.push_view(View::unescaped_unchecked("<hr>"));
                parts.push_view(View::empty());
            });
            assert_eq!(outer.render(cx), "<hr>");
        });
    }

    #[test]
    fn filled_view_slot_renders_the_resolved_view() {
        in_scope(async |cx| {
            let (placeholder, slot) = reserve();
            // The outer view splices the placeholder before the child exists.
            let outer = build_sync(|parts| {
                parts.push_str_unescaped("<p>");
                parts.push_view(placeholder.clone());
                parts.push_str_unescaped("</p>");
            });

            let child = build_sync(|parts| {
                parts.push_str("a < b");
            });
            slot.fill(child);

            assert_eq!(outer.render(cx), "<p>a &lt; b</p>");
            assert_eq!(placeholder.render(cx), "a &lt; b");
        });
    }

    #[test]
    fn static_views_fill_a_slot_like_scoped_ones() {
        in_scope(async |cx| {
            let (placeholder, slot) = reserve();
            slot.fill(View::unescaped_unchecked("<hr>"));
            assert_eq!(placeholder.render(cx), "<hr>");

            let (placeholder, slot) = reserve();
            slot.fill(View::empty());
            assert_eq!(placeholder.render(cx), "");
        });
    }

    #[test]
    #[should_panic(expected = "before it was filled")]
    fn rendering_an_unfilled_placeholder_panics() {
        in_scope(async |cx| {
            let (placeholder, _slot) = reserve();
            placeholder.render(cx)
        });
    }

    #[test]
    #[should_panic(expected = "tried to fill a view slot twice")]
    fn filling_a_slot_twice_panics() {
        in_scope(async |_cx| {
            let (_placeholder, slot) = reserve();
            slot.fill(View::empty());
            slot.fill(View::empty());
        });
    }

    #[test]
    #[should_panic(expected = "outside the `view!` invocation it was reserved in")]
    fn filling_a_slot_in_a_different_root_build_panics() {
        let slot = in_scope(async |_cx| reserve().1);
        in_scope(async |_cx| slot.fill(View::empty()));
    }

    #[test]
    fn size_hint_accumulates_across_splices() {
        in_scope(async |_cx| {
            let inner = build_sync(|parts| {
                parts.push_str_unescaped("12345678");
            });

            let outer = build_sync(|parts| {
                parts.push_view(inner.clone());
                parts.push_view(inner);
                parts.push_view(View::unescaped_unchecked("<hr>"));
            });
            let ViewRepr::Scoped { size_hint, .. } = outer.repr() else {
                panic!("expected a scoped view");
            };
            assert_eq!(size_hint, 8 + 8 + 4);
        });
    }

    #[test]
    fn build_sync_outside_a_root_build_owns_its_arena() {
        let view = build_sync(|parts| {
            parts.push_str("a < b");
        });
        assert!(matches!(view.repr, ViewRepr::Owned { .. }));
        assert_eq!(view.render(&Cx::default()), "a &lt; b");
    }

    #[test]
    #[should_panic(expected = "no view is building")]
    fn emitting_a_block_outside_a_root_build_panics() {
        crate::internal::block(&Cx::default(), |_b| {});
    }

    #[test]
    #[should_panic(expected = "no view is building")]
    fn rendering_an_escaped_nested_view_panics() {
        let view = in_scope(async |_cx| build_sync(|_parts| {}));
        view.render(&Cx::default());
    }

    #[test]
    #[should_panic(expected = "outside the `view!` invocation it was built in")]
    fn rendering_a_nested_view_in_a_different_root_build_panics() {
        let view = in_scope(async |_cx| build_sync(|_parts| {}));
        in_scope(async |cx| view.render(cx));
    }

    #[test]
    #[should_panic(expected = "outside the `view!` invocation it was built in")]
    fn splicing_a_nested_view_from_a_different_root_build_panics() {
        let view = in_scope(async |_cx| build_sync(|_parts| {}));
        in_scope(async |_cx| {
            build_sync(|parts| {
                parts.push_view(view);
            })
        });
    }

    #[cfg(feature = "http")]
    mod response {
        use http::{
            HeaderMap, HeaderName, HeaderValue, StatusCode,
            header::{CACHE_CONTROL, SET_COOKIE},
        };

        use super::*;
        use crate::NodeViewParts;

        fn push_node(cx: &Cx, parts: &mut PartsWriter<'_>, value: impl NodeViewParts) {
            value.into_view_parts(cx, parts);
        }

        #[test]
        fn status_code_is_recorded_and_renders_nothing() {
            in_scope(async |cx| {
                let view = build_sync(|parts| {
                    push_node(cx, parts, "a");
                    push_node(cx, parts, StatusCode::NOT_FOUND);
                    push_node(cx, parts, "b");
                });

                let rendered = view.render_response(cx);
                assert_eq!(rendered.html, "ab");
                assert_eq!(rendered.status_code, Some(StatusCode::NOT_FOUND));
                assert!(rendered.headers.is_empty());
            });
        }

        #[test]
        fn render_response_without_declarations_is_empty() {
            in_scope(async |cx| {
                let view = build_sync(|parts| {
                    push_node(cx, parts, "a");
                });

                let rendered = view.render_response(cx);
                assert_eq!(rendered.html, "a");
                assert_eq!(rendered.status_code, None);
                assert!(rendered.headers.is_empty());
            });
        }

        #[test]
        fn render_discards_declarations() {
            in_scope(async |cx| {
                let view = build_sync(|parts| {
                    push_node(cx, parts, StatusCode::NOT_FOUND);
                    push_node(
                        cx,
                        parts,
                        (CACHE_CONTROL, HeaderValue::from_static("no-store")),
                    );
                    push_node(cx, parts, "a");
                });

                assert_eq!(view.render(cx), "a");
            });
        }

        #[test]
        fn first_status_code_wins() {
            in_scope(async |cx| {
                let view = build_sync(|parts| {
                    push_node(cx, parts, StatusCode::NOT_FOUND);
                    push_node(cx, parts, StatusCode::OK);
                });

                let rendered = view.render_response(cx);
                assert_eq!(rendered.status_code, Some(StatusCode::NOT_FOUND));
            });
        }

        #[test]
        fn first_mention_of_a_header_name_wins() {
            in_scope(async |cx| {
                let view = build_sync(|parts| {
                    push_node(
                        cx,
                        parts,
                        (CACHE_CONTROL, HeaderValue::from_static("no-store")),
                    );
                    let mut later = HeaderMap::new();
                    later.insert(CACHE_CONTROL, HeaderValue::from_static("max-age=60"));
                    later.insert(
                        HeaderName::from_static("x-extra"),
                        HeaderValue::from_static("1"),
                    );
                    push_node(cx, parts, later);
                });

                let rendered = view.render_response(cx);
                assert_eq!(rendered.headers[CACHE_CONTROL], "no-store");
                assert_eq!(rendered.headers["x-extra"], "1");
            });
        }

        #[test]
        fn one_map_keeps_all_values_for_a_name() {
            in_scope(async |cx| {
                let mut first = HeaderMap::new();
                first.append(SET_COOKIE, HeaderValue::from_static("a=1"));
                first.append(SET_COOKIE, HeaderValue::from_static("b=2"));
                let mut later = HeaderMap::new();
                later.insert(SET_COOKIE, HeaderValue::from_static("c=3"));

                let view = build_sync(|parts| {
                    push_node(cx, parts, first);
                    push_node(cx, parts, later);
                });

                let rendered = view.render_response(cx);
                let cookies: Vec<_> = rendered.headers.get_all(SET_COOKIE).iter().collect();
                assert_eq!(cookies, ["a=1", "b=2"]);
            });
        }

        #[test]
        fn placement_decides_precedence_across_nested_views() {
            in_scope(async |cx| {
                let inner = build_sync(|parts| {
                    push_node(cx, parts, StatusCode::NOT_FOUND);
                    push_node(cx, parts, "inner");
                });

                // A status code before the nested view overrides it.
                let outer = build_sync(|parts| {
                    push_node(cx, parts, StatusCode::FORBIDDEN);
                    parts.push_view(inner.clone());
                });
                let rendered = outer.render_response(cx);
                assert_eq!(rendered.status_code, Some(StatusCode::FORBIDDEN));

                // A status code after the nested view is only a fallback.
                let outer = build_sync(|parts| {
                    parts.push_view(inner);
                    push_node(cx, parts, StatusCode::FORBIDDEN);
                });
                let rendered = outer.render_response(cx);
                assert_eq!(rendered.status_code, Some(StatusCode::NOT_FOUND));
            });
        }
    }
}
