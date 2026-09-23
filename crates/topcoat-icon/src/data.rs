use topcoat_view::{ViewHandle, svg::ViewBox};

/// The data of an SVG icon: its view box and its body markup.
///
/// The body is the markup inside the `<svg>` element, without the element
/// itself. Render the icon with the [`icon`](crate::icon) component, which adds
/// the `<svg>` element around the body.
#[derive(Debug, Clone)]
pub struct IconData {
    view_box: ViewBox,
    body: ViewHandle,
}

impl IconData {
    /// Creates an icon from its view box and a body view.
    #[must_use]
    pub fn new(view_box: ViewBox, body: ViewHandle) -> Self {
        Self { view_box, body }
    }

    /// Creates an icon from its view box and a body of raw SVG markup.
    ///
    /// The body is rendered as-is. It is not escaped or checked for syntax
    /// errors, so only pass markup you trust. This is a `const fn`, so the
    /// result can be stored in a `const`.
    #[must_use]
    pub const fn unescaped_unchecked(view_box: ViewBox, body: &'static str) -> Self {
        Self {
            view_box,
            body: ViewHandle::unescaped_unchecked(body),
        }
    }

    /// The icon's view box.
    #[must_use]
    pub const fn view_box(&self) -> ViewBox {
        self.view_box
    }

    /// Consumes the icon and returns its body view.
    #[must_use]
    pub fn into_body(self) -> ViewHandle {
        self.body
    }
}
