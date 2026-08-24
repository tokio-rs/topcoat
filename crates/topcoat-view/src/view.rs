use std::{
    cell::Cell,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures_core::Stream;
use futures_util::future::poll_fn;
#[cfg(feature = "http")]
use http::{HeaderMap, StatusCode};
use pin_project_lite::pin_project;
use topcoat_core::context::Cx;
use topcoat_core::error::Result;

use crate::{
    Formatter,
    buffer::{InstructionPtr, Renderer, ViewBuffer, ViewBufferId, ViewBufferScope},
};

pub struct ViewChunk {
    id: u64,
    pub view: ViewHandle, // TODO: not pub
}

impl ViewChunk {
    /// Wraps a view in a content chunk.
    ///
    /// The id targets the position a later chunk swaps into; content chunks
    /// carry `0` until the live swap protocol assigns real ids.
    pub(crate) fn new(view: ViewHandle) -> Self {
        Self { id: 0, view }
    }
}

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
/// The outermost `view!` invocation allocates an instruction buffer that the
/// returned view owns; every `view!` nested inside it, such as a component
/// body, appends to that same buffer and returns a cheap handle into it. An
/// owned view is a self-contained value: it can be stored, sent across
/// tasks, spliced into another view, and rendered anywhere. A nested handle
/// only lives while its enclosing invocation builds; one that escapes it
/// panics when used.
#[derive(Debug, Default, Clone)]
pub struct ViewHandle {
    repr: ViewRepr,
}

/// The kinds of view: a static string independent of any buffer, a handle
/// into the buffer of an enclosing `view!` invocation still building, or an
/// owned view carrying its own buffer.
#[derive(Debug, Clone)]
pub(crate) enum ViewRepr {
    /// Trusted static markup rendered verbatim, independent of any buffer.
    Static(&'static str),
    /// An instruction block in the still building buffer identified by
    /// `buffer`, starting at `entry`.
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
    pub(crate) fn from_scope(
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

    /// Seals a root build: the view takes ownership of the buffer its
    /// instructions were appended to. With no buffer, the build was nested
    /// and the view stays a handle into the enclosing invocation's buffer.
    ///
    /// A static view carries no instructions, so it passes through and the
    /// buffer is dropped.
    ///
    /// # Panics
    ///
    /// Panics if the view's instructions live in a different buffer.
    pub(crate) fn seal(self, buffer: Option<ViewBuffer>) -> Self {
        let Some(buffer) = buffer else {
            return self;
        };
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
    /// Panics if the view is a nested handle that escaped the `view!`
    /// invocation it was built in, or if a dynamic attribute key or element
    /// name in the view contains a character that could break out of the
    /// identifier.
    #[must_use]
    #[track_caller]
    pub fn render(self, cx: &Cx) -> String {
        match self.repr {
            ViewRepr::Static(body) => body.to_owned(),
            ViewRepr::Scoped {
                buffer,
                entry,
                size_hint,
            } => {
                let mut html = String::with_capacity(size_hint);
                let mut f = Formatter::new(&mut html);
                Self::execute(buffer, entry, cx, &mut f);
                html
            }
            ViewRepr::Owned {
                buffer,
                entry,
                size_hint,
            } => {
                let mut html = String::with_capacity(size_hint);
                let mut f = Formatter::new(&mut html);
                Renderer::new(&buffer, entry).execute(cx, &mut f);
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
                buffer,
                entry,
                size_hint,
            } => {
                let mut html = String::with_capacity(size_hint);
                let mut f = Formatter::new(&mut html);
                Self::execute(buffer, entry, cx, &mut f);
                let (status_code, headers) = f.into_recorded();
                RenderedResponse {
                    html,
                    status_code,
                    headers,
                }
            }
            ViewRepr::Owned {
                buffer,
                entry,
                size_hint,
            } => {
                let mut html = String::with_capacity(size_hint);
                let mut f = Formatter::new(&mut html);
                Renderer::new(&buffer, entry).execute(cx, &mut f);
                let (status_code, headers) = f.into_recorded();
                RenderedResponse {
                    html,
                    status_code,
                    headers,
                }
            }
        }
    }

    /// Executes a nested view handle's instruction block against the buffer
    /// of the enclosing `view!` invocation still building it.
    #[track_caller]
    fn execute(buffer: ViewBufferId, entry: InstructionPtr, cx: &Cx, f: &mut Formatter<'_>) {
        ViewBufferScope::with(|active| {
            assert!(
                active.id() == buffer,
                "tried to render a view outside the `view!` invocation it was built in",
            );
            Renderer::new(active, entry).execute(cx, f);
        });
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
    use std::{
        future::Future,
        pin::pin,
        task::{Context, Poll, Waker},
    };

    use super::*;
    use crate::{
        PartsWriter,
        internal::{build, build_sync, reserve, write_block},
    };

    /// Runs `f` with a request context inside a fresh view scope.
    fn in_scope<R>(f: impl AsyncFnOnce(&Cx) -> R) -> R {
        block_on(ViewBufferScope::scope(async { f(&Cx::default()).await })).0
    }

    /// Builds a view in one synchronous burst from the parts `f` pushes.
    ///
    /// Appends to the enclosing buffer inside a root build and owns a buffer
    /// of its own outside one.
    fn sync(f: impl FnOnce(&mut PartsWriter<'_>)) -> ViewHandle {
        build_sync(|| write_block(f))
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

    #[test]
    fn static_views_render_without_a_scope() {
        assert_eq!(ViewHandle::empty().render(&Cx::default()), "");
        let view = ViewHandle::unescaped_unchecked("<b>raw</b>");
        assert_eq!(view.render(&Cx::default()), "<b>raw</b>");
    }

    #[test]
    fn push_view_splices_nested_views() {
        in_scope(async |cx| {
            let inner = sync(|parts| {
                parts.push_str("a < b");
            });

            let outer = sync(|parts| {
                parts.push_str_unescaped("<p>");
                parts.push_view(inner);
                parts.push_str_unescaped("</p>");
            });
            assert_eq!(outer.render(cx), "<p>a &lt; b</p>");
        });
    }

    #[test]
    fn document_order_follows_splice_order_not_buffer_order() {
        in_scope(async |cx| {
            // Built in reverse: `second` occupies earlier buffer addresses.
            let second = sync(|parts| {
                parts.push_str("B");
            });
            let first = sync(|parts| {
                parts.push_str("A");
            });

            let outer = sync(|parts| {
                parts.push_view(first);
                parts.push_view(second);
            });
            assert_eq!(outer.render(cx), "AB");
        });
    }

    #[test]
    fn owned_views_render_without_an_active_build() {
        let view = sync(|parts| {
            parts.push_str("a < b");
        });
        assert_eq!(view.render(&Cx::default()), "a &lt; b");
    }

    #[test]
    fn owned_views_splice_across_buffers() {
        let inner = sync(|parts| {
            parts.push_str("a < b");
        });

        let outer = sync(|parts| {
            parts.push_str_unescaped("<p>");
            parts.push_view(inner);
            parts.push_str_unescaped("</p>");
        });
        assert_eq!(outer.render(&Cx::default()), "<p>a &lt; b</p>");
    }

    #[test]
    fn owned_views_splice_into_an_active_build() {
        let inner = sync(|parts| {
            parts.push_str("a < b");
        });

        in_scope(async |cx| {
            let outer = sync(|parts| {
                parts.push_str_unescaped("<p>");
                parts.push_view(inner);
                parts.push_str_unescaped("</p>");
            });
            assert_eq!(outer.render(cx), "<p>a &lt; b</p>");
        });
    }

    #[test]
    fn owned_views_fill_a_slot_like_nested_ones() {
        let inner = sync(|parts| {
            parts.push_str("a < b");
        });

        in_scope(async |cx| {
            let (placeholder, slot) = reserve();
            slot.fill(inner);
            assert_eq!(placeholder.render(cx), "a &lt; b");
        });
    }

    #[test]
    fn nested_root_invocations_append_to_the_enclosing_buffer() {
        in_scope(async |cx| {
            let inner = build(async {
                Ok(sync(|parts| {
                    parts.push_str("x");
                }))
            })
            .await
            .expect("the build is infallible");
            assert!(matches!(inner.repr, ViewRepr::Scoped { .. }));

            let outer = sync(|parts| {
                parts.push_view(inner);
            });
            assert_eq!(outer.render(cx), "x");
        });
    }

    #[test]
    fn owned_views_are_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>(_value: &T) {}

        let view = sync(|parts| {
            parts.push_str("x");
        });
        assert_send_sync(&view);
    }

    #[test]
    fn static_views_are_spliced_verbatim() {
        in_scope(async |cx| {
            let outer = sync(|parts| {
                parts.push_view(ViewHandle::unescaped_unchecked("<hr>"));
                parts.push_view(ViewHandle::empty());
            });
            assert_eq!(outer.render(cx), "<hr>");
        });
    }

    #[test]
    fn filled_view_slot_renders_the_resolved_view() {
        in_scope(async |cx| {
            let (placeholder, slot) = reserve();
            // The outer view splices the placeholder before the child exists.
            let outer = sync(|parts| {
                parts.push_str_unescaped("<p>");
                parts.push_view(placeholder.clone());
                parts.push_str_unescaped("</p>");
            });

            let child = sync(|parts| {
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
            slot.fill(ViewHandle::unescaped_unchecked("<hr>"));
            assert_eq!(placeholder.render(cx), "<hr>");

            let (placeholder, slot) = reserve();
            slot.fill(ViewHandle::empty());
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
            slot.fill(ViewHandle::empty());
            slot.fill(ViewHandle::empty());
        });
    }

    #[test]
    #[should_panic(expected = "outside the `view!` invocation it was reserved in")]
    fn filling_a_slot_in_a_different_root_build_panics() {
        let slot = in_scope(async |_cx| reserve().1);
        in_scope(async |_cx| slot.fill(ViewHandle::empty()));
    }

    #[test]
    fn size_hint_accumulates_across_splices() {
        in_scope(async |_cx| {
            let inner = sync(|parts| {
                parts.push_str_unescaped("12345678");
            });

            let outer = sync(|parts| {
                parts.push_view(inner.clone());
                parts.push_view(inner);
                parts.push_view(ViewHandle::unescaped_unchecked("<hr>"));
            });
            let ViewRepr::Scoped { size_hint, .. } = outer.repr() else {
                panic!("expected a scoped view");
            };
            assert_eq!(size_hint, 8 + 8 + 4);
        });
    }

    #[test]
    fn build_sync_outside_a_root_build_owns_its_buffer() {
        let view = sync(|parts| {
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
        let view = in_scope(async |_cx| sync(|_parts| {}));
        let _ = view.render(&Cx::default());
    }

    #[test]
    #[should_panic(expected = "outside the `view!` invocation it was built in")]
    fn rendering_a_nested_view_in_a_different_root_build_panics() {
        let view = in_scope(async |_cx| sync(|_parts| {}));
        in_scope(async |cx| view.render(cx));
    }

    #[test]
    #[should_panic(expected = "outside the `view!` invocation it was built in")]
    fn splicing_a_nested_view_from_a_different_root_build_panics() {
        let view = in_scope(async |_cx| sync(|_parts| {}));
        in_scope(async |_cx| {
            sync(|parts| {
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
                let view = sync(|parts| {
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
                let view = sync(|parts| {
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
                let view = sync(|parts| {
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
                let view = sync(|parts| {
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
                let view = sync(|parts| {
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

                let view = sync(|parts| {
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
                let inner = sync(|parts| {
                    push_node(cx, parts, StatusCode::NOT_FOUND);
                    push_node(cx, parts, "inner");
                });

                // A status code before the nested view overrides it.
                let outer = sync(|parts| {
                    push_node(cx, parts, StatusCode::FORBIDDEN);
                    parts.push_view(inner.clone());
                });
                let rendered = outer.render_response(cx);
                assert_eq!(rendered.status_code, Some(StatusCode::FORBIDDEN));

                // A status code after the nested view is only a fallback.
                let outer = sync(|parts| {
                    parts.push_view(inner);
                    push_node(cx, parts, StatusCode::FORBIDDEN);
                });
                let rendered = outer.render_response(cx);
                assert_eq!(rendered.status_code, Some(StatusCode::NOT_FOUND));
            });
        }
    }
}
