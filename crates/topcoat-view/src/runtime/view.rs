use topcoat_core::context::Cx;

use crate::runtime::{Formatter, Fragment};

/// A piece of HTML content.
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
#[derive(Debug, Clone)]
pub struct View {
    content: String,
}

impl View {
    #[inline]
    pub fn new(content: String) -> Self {
        Self { content }
    }
}

impl Fragment for View {
    fn fmt(&self, cx: &Cx, f: &mut Formatter<'_>) {
        self.content.fmt(cx, f);
    }

    #[inline]
    fn size_hint(&self) -> usize {
        self.content.len()
    }
}

#[cfg(feature = "axum")]
impl axum::response::IntoResponse for View {
    fn into_response(self) -> axum::response::Response {
        axum::response::Html(self.content).into_response()
    }
}
