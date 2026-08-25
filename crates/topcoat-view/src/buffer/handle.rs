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
pub(crate) enum ViewRepr {
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

