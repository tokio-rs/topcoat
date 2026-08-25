use std::sync::Arc;

#[cfg(feature = "http")]
use http::{HeaderMap, StatusCode};
use topcoat_core::context::Cx;

use crate::{
    Formatter,
    buffer::{InstructionPtr, Renderer, ViewBuffer, ViewBufferId},
};

/// A self-contained piece of HTML content.
///
/// A view handle may contain multiple sibling nodes, but opened tags must be closed
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
/// A handle is either self-contained or nested. A self-contained handle
/// carries everything it needs to render: it can be stored, sent across
/// tasks, spliced into another view, and rendered anywhere. A nested handle
/// is what a [`View`](crate::View) returns as its content: it points into
/// the buffer the view was polled with and only means something to the
/// caller holding that buffer, who splices it into content of its own.
#[derive(Debug, Default, Clone)]
pub struct ViewHandle {
    repr: ViewRepr,
}

/// The kinds of view: a static string independent of any buffer, a handle
/// into a buffer someone else holds, or an owned view carrying its own
/// buffer.
#[derive(Debug, Clone)]
pub(super) enum ViewRepr {
    /// Trusted static markup rendered verbatim, independent of any buffer.
    Static(&'static str),
    /// An instruction block in the buffer identified by `buffer`, starting
    /// at `entry`.
    Scoped {
        buffer: ViewBufferId,
        entry: InstructionPtr,
        /// An estimate of the number of bytes the block writes when
        /// rendered, accumulated while the view was built.
        size_hint: usize,
    },
    /// An instruction block starting at `entry` in a buffer the view holds
    /// on to itself.
    Owned {
        buffer: Arc<ViewBuffer>,
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

impl ViewHandle {
    /// Creates the handle for an instruction block built in the buffer
    /// identified by `buffer`, estimated to write `size_hint` bytes.
    #[inline]
    pub(super) fn from_scope(
        buffer: ViewBufferId,
        entry: InstructionPtr,
        size_hint: usize,
    ) -> Self {
        Self {
            repr: ViewRepr::Scoped {
                buffer,
                entry,
                size_hint,
            },
        }
    }

    /// Unwraps the view into its representation.
    #[inline]
    pub(super) fn repr(self) -> ViewRepr {
        self.repr
    }

    /// Returns an estimate of the number of bytes the view writes when
    /// rendered.
    #[inline]
    pub(super) fn size_hint(&self) -> usize {
        match &self.repr {
            ViewRepr::Static(body) => body.len(),
            ViewRepr::Scoped { size_hint, .. } | ViewRepr::Owned { size_hint, .. } => *size_hint,
        }
    }

    /// Makes the view self-contained by taking ownership of the buffer its
    /// instructions were appended to.
    ///
    /// A static view carries no instructions, so it passes through and the
    /// buffer is dropped.
    ///
    /// # Panics
    ///
    /// Panics if the view's instructions live in a different buffer.
    pub(crate) fn seal(self, buffer: ViewBuffer) -> Self {
        match self.repr {
            ViewRepr::Static(_) | ViewRepr::Owned { .. } => self,
            ViewRepr::Scoped {
                buffer: id,
                entry,
                size_hint,
            } => {
                assert!(
                    id == buffer.id(),
                    "tried to seal a view into a buffer it was not built in",
                );
                Self {
                    repr: ViewRepr::Owned {
                        buffer: Arc::new(buffer),
                        entry,
                        size_hint,
                    },
                }
            }
        }
    }

    /// Returns a `ViewHandle` that renders to an empty string.
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
    /// Panics if the view is a nested handle, or if a dynamic attribute key
    /// or element name in the view contains a character that could break
    /// out of the identifier.
    #[must_use]
    #[track_caller]
    pub fn render(self, cx: &Cx) -> String {
        let mut html = String::with_capacity(self.size_hint());
        self.render_into(cx, &mut Formatter::new(&mut html));
        html
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
    /// Panics if the view is a nested handle, or if a dynamic attribute key
    /// or element name in the view contains a character that could break
    /// out of the identifier.
    #[cfg(feature = "http")]
    #[must_use]
    #[track_caller]
    pub fn render_response(self, cx: &Cx) -> RenderedResponse {
        let mut html = String::with_capacity(self.size_hint());
        let mut f = Formatter::new(&mut html);
        self.render_into(cx, &mut f);
        let (status_code, headers) = f.into_recorded();
        RenderedResponse {
            html,
            status_code,
            headers,
        }
    }

    /// Writes the view's output through `f`.
    #[track_caller]
    fn render_into(self, cx: &Cx, f: &mut Formatter<'_>) {
        match self.repr {
            ViewRepr::Static(body) => f.write_str(body),
            ViewRepr::Scoped { .. } => {
                panic!("tried to render a nested view handle; only a self-contained view renders")
            }
            ViewRepr::Owned { buffer, entry, .. } => {
                Renderer::new(&buffer, entry).execute(cx, f);
            }
        }
    }
}

/// The output of rendering a [`ViewHandle`] for an HTTP response.
///
/// Returned by [`ViewHandle::render_response`]: the rendered HTML alongside the
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PartsWriter;

    /// Appends a nested view to `buffer` in one synchronous burst from the
    /// parts `f` pushes.
    fn nested(buffer: &mut ViewBuffer, f: impl FnOnce(&mut PartsWriter<'_>)) -> ViewHandle {
        buffer.block(&Cx::default(), |b| f(b.parts()))
    }

    /// Builds a self-contained view in one synchronous burst from the parts
    /// `f` pushes.
    fn owned(f: impl FnOnce(&mut PartsWriter<'_>)) -> ViewHandle {
        let mut buffer = ViewBuffer::new();
        nested(&mut buffer, f).seal(buffer)
    }

    #[test]
    fn static_views_render_without_a_buffer() {
        assert_eq!(ViewHandle::empty().render(&Cx::default()), "");
        let view = ViewHandle::unescaped_unchecked("<b>raw</b>");
        assert_eq!(view.render(&Cx::default()), "<b>raw</b>");
    }

    #[test]
    fn push_view_splices_nested_views() {
        let mut buffer = ViewBuffer::new();
        let inner = nested(&mut buffer, |parts| {
            parts.push_str("a < b");
        });
        let outer = nested(&mut buffer, |parts| {
            parts.push_str_unescaped("<p>");
            parts.push_view_handle(inner);
            parts.push_str_unescaped("</p>");
        });
        assert_eq!(outer.seal(buffer).render(&Cx::default()), "<p>a &lt; b</p>");
    }

    #[test]
    fn document_order_follows_splice_order_not_buffer_order() {
        let mut buffer = ViewBuffer::new();
        // Built in reverse: `second` occupies earlier buffer addresses.
        let second = nested(&mut buffer, |parts| {
            parts.push_str("B");
        });
        let first = nested(&mut buffer, |parts| {
            parts.push_str("A");
        });
        let outer = nested(&mut buffer, |parts| {
            parts.push_view_handle(first);
            parts.push_view_handle(second);
        });
        assert_eq!(outer.seal(buffer).render(&Cx::default()), "AB");
    }

    #[test]
    fn sealed_views_own_their_buffer() {
        let view = owned(|parts| {
            parts.push_str("a < b");
        });
        assert!(matches!(view.repr, ViewRepr::Owned { .. }));
        assert_eq!(view.render(&Cx::default()), "a &lt; b");
    }

    #[test]
    fn owned_views_splice_across_buffers() {
        let inner = owned(|parts| {
            parts.push_str("a < b");
        });
        let outer = owned(|parts| {
            parts.push_str_unescaped("<p>");
            parts.push_view_handle(inner);
            parts.push_str_unescaped("</p>");
        });
        assert_eq!(outer.render(&Cx::default()), "<p>a &lt; b</p>");
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
        let outer = owned(|parts| {
            parts.push_view_handle(ViewHandle::unescaped_unchecked("<hr>"));
            parts.push_view_handle(ViewHandle::empty());
        });
        assert_eq!(outer.render(&Cx::default()), "<hr>");
    }

    #[test]
    fn filled_view_slot_renders_the_resolved_view() {
        let mut buffer = ViewBuffer::new();
        let (placeholder, slot) = buffer.reserve_view();
        // The outer view splices the placeholder before the child exists.
        let outer = nested(&mut buffer, |parts| {
            parts.push_str_unescaped("<p>");
            parts.push_view_handle(placeholder);
            parts.push_str_unescaped("</p>");
        });
        let child = nested(&mut buffer, |parts| {
            parts.push_str("a < b");
        });
        buffer.fill_view(slot, child);
        assert_eq!(outer.seal(buffer).render(&Cx::default()), "<p>a &lt; b</p>");
    }

    #[test]
    fn a_placeholder_renders_the_view_filling_its_slot() {
        let mut buffer = ViewBuffer::new();
        let (placeholder, slot) = buffer.reserve_view();
        let child = nested(&mut buffer, |parts| {
            parts.push_str("a < b");
        });
        buffer.fill_view(slot, child);
        assert_eq!(placeholder.seal(buffer).render(&Cx::default()), "a &lt; b");
    }

    #[test]
    fn owned_views_fill_a_slot_like_nested_ones() {
        let inner = owned(|parts| {
            parts.push_str("a < b");
        });
        let mut buffer = ViewBuffer::new();
        let (placeholder, slot) = buffer.reserve_view();
        buffer.fill_view(slot, inner);
        assert_eq!(placeholder.seal(buffer).render(&Cx::default()), "a &lt; b");
    }

    #[test]
    fn static_views_fill_a_slot_like_nested_ones() {
        let mut buffer = ViewBuffer::new();
        let (placeholder, slot) = buffer.reserve_view();
        buffer.fill_view(slot, ViewHandle::unescaped_unchecked("<hr>"));
        assert_eq!(placeholder.seal(buffer).render(&Cx::default()), "<hr>");

        let mut buffer = ViewBuffer::new();
        let (placeholder, slot) = buffer.reserve_view();
        buffer.fill_view(slot, ViewHandle::empty());
        assert_eq!(placeholder.seal(buffer).render(&Cx::default()), "");
    }

    #[test]
    #[should_panic(expected = "before it was filled")]
    fn rendering_an_unfilled_placeholder_panics() {
        let mut buffer = ViewBuffer::new();
        let (placeholder, _slot) = buffer.reserve_view();
        let _ = placeholder.seal(buffer).render(&Cx::default());
    }

    #[test]
    #[should_panic(expected = "tried to fill a view slot twice")]
    fn filling_a_slot_twice_panics() {
        let mut buffer = ViewBuffer::new();
        let (_placeholder, slot) = buffer.reserve_view();
        buffer.fill_view(slot, ViewHandle::empty());
        buffer.fill_view(slot, ViewHandle::empty());
    }

    #[test]
    #[should_panic(expected = "outside the `view!` invocation it was reserved in")]
    fn filling_a_slot_in_a_different_buffer_panics() {
        let mut reserved_in = ViewBuffer::new();
        let (_placeholder, slot) = reserved_in.reserve_view();
        let mut other = ViewBuffer::new();
        other.fill_view(slot, ViewHandle::empty());
    }

    #[test]
    fn size_hint_accumulates_across_splices() {
        let mut buffer = ViewBuffer::new();
        let inner = nested(&mut buffer, |parts| {
            parts.push_str_unescaped("12345678");
        });
        let outer = nested(&mut buffer, |parts| {
            parts.push_view_handle(inner.clone());
            parts.push_view_handle(inner);
            parts.push_view_handle(ViewHandle::unescaped_unchecked("<hr>"));
        });
        let ViewRepr::Scoped { size_hint, .. } = outer.repr() else {
            panic!("expected a nested view");
        };
        assert_eq!(size_hint, 8 + 8 + 4);
    }

    #[test]
    #[should_panic(expected = "tried to render a nested view handle")]
    fn rendering_a_nested_view_panics() {
        let mut buffer = ViewBuffer::new();
        let view = nested(&mut buffer, |_parts| {});
        let _ = view.render(&Cx::default());
    }

    #[test]
    #[should_panic(expected = "outside the `view!` invocation it was built in")]
    fn splicing_a_nested_view_from_a_different_buffer_panics() {
        let mut built_in = ViewBuffer::new();
        let view = nested(&mut built_in, |_parts| {});
        let mut other = ViewBuffer::new();
        nested(&mut other, |parts| {
            parts.push_view_handle(view);
        });
    }

    #[test]
    #[should_panic(expected = "tried to seal a view into a buffer it was not built in")]
    fn sealing_a_view_into_a_different_buffer_panics() {
        let mut built_in = ViewBuffer::new();
        let view = nested(&mut built_in, |_parts| {});
        let _ = view.seal(ViewBuffer::new());
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
            let cx = &Cx::default();
            let view = owned(|parts| {
                push_node(cx, parts, "a");
                push_node(cx, parts, StatusCode::NOT_FOUND);
                push_node(cx, parts, "b");
            });

            let rendered = view.render_response(cx);
            assert_eq!(rendered.html, "ab");
            assert_eq!(rendered.status_code, Some(StatusCode::NOT_FOUND));
            assert!(rendered.headers.is_empty());
        }

        #[test]
        fn render_response_without_declarations_is_empty() {
            let cx = &Cx::default();
            let view = owned(|parts| {
                push_node(cx, parts, "a");
            });

            let rendered = view.render_response(cx);
            assert_eq!(rendered.html, "a");
            assert_eq!(rendered.status_code, None);
            assert!(rendered.headers.is_empty());
        }

        #[test]
        fn render_discards_declarations() {
            let cx = &Cx::default();
            let view = owned(|parts| {
                push_node(cx, parts, StatusCode::NOT_FOUND);
                push_node(
                    cx,
                    parts,
                    (CACHE_CONTROL, HeaderValue::from_static("no-store")),
                );
                push_node(cx, parts, "a");
            });

            assert_eq!(view.render(cx), "a");
        }

        #[test]
        fn first_status_code_wins() {
            let cx = &Cx::default();
            let view = owned(|parts| {
                push_node(cx, parts, StatusCode::NOT_FOUND);
                push_node(cx, parts, StatusCode::OK);
            });

            let rendered = view.render_response(cx);
            assert_eq!(rendered.status_code, Some(StatusCode::NOT_FOUND));
        }

        #[test]
        fn first_mention_of_a_header_name_wins() {
            let cx = &Cx::default();
            let view = owned(|parts| {
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
        }

        #[test]
        fn one_map_keeps_all_values_for_a_name() {
            let cx = &Cx::default();
            let mut first = HeaderMap::new();
            first.append(SET_COOKIE, HeaderValue::from_static("a=1"));
            first.append(SET_COOKIE, HeaderValue::from_static("b=2"));
            let mut later = HeaderMap::new();
            later.insert(SET_COOKIE, HeaderValue::from_static("c=3"));

            let view = owned(|parts| {
                push_node(cx, parts, first);
                push_node(cx, parts, later);
            });

            let rendered = view.render_response(cx);
            let cookies: Vec<_> = rendered.headers.get_all(SET_COOKIE).iter().collect();
            assert_eq!(cookies, ["a=1", "b=2"]);
        }

        #[test]
        fn placement_decides_precedence_across_nested_views() {
            let cx = &Cx::default();
            let inner = owned(|parts| {
                push_node(cx, parts, StatusCode::NOT_FOUND);
                push_node(cx, parts, "inner");
            });

            // A status code before the nested view overrides it.
            let outer = owned(|parts| {
                push_node(cx, parts, StatusCode::FORBIDDEN);
                parts.push_view_handle(inner.clone());
            });
            let rendered = outer.render_response(cx);
            assert_eq!(rendered.status_code, Some(StatusCode::FORBIDDEN));

            // A status code after the nested view is only a fallback.
            let outer = owned(|parts| {
                parts.push_view_handle(inner);
                push_node(cx, parts, StatusCode::FORBIDDEN);
            });
            let rendered = outer.render_response(cx);
            assert_eq!(rendered.status_code, Some(StatusCode::NOT_FOUND));
        }
    }
}
