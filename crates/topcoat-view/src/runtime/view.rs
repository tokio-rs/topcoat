use std::borrow::Cow;
use std::fmt;

/// A rendered piece of HTML content.
///
/// A `View` contains a self-contained HTML fragment where all tags are fully
/// closed. This means it can contain multiple sibling elements, but every
/// opened tag must be closed so that the fragment can be safely nested inside
/// a larger HTML document without breaking the surrounding markup.
///
/// ```html
/// <!-- Valid: all tags are closed, safe to nest -->
/// <div>Hello</div>
/// <p>World</p>
///
/// <!-- Invalid: unclosed tag would corrupt the parent document -->
/// <div>Hello
/// ```
pub struct View {
    pub(super) buf: Cow<'static, str>,
}

impl View {
    /// Creates a new `View` from the given HTML content.
    ///
    /// The caller is responsible for ensuring the HTML fragment is
    /// well-formed with all tags properly closed.
    #[inline]
    pub fn new(buf: impl Into<Cow<'static, str>>) -> Self {
        Self { buf: buf.into() }
    }
}

impl fmt::Display for View {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.buf)
    }
}

#[cfg(feature = "axum")]
impl axum::response::IntoResponse for View {
    fn into_response(self) -> axum::response::Response {
        axum::response::Html(self.buf.into_owned()).into_response()
    }
}
