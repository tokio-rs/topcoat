use topcoat_view::{ViewHandle, svg::ViewBox};

/// An SVG icon's view box and body markup.
///
/// The body is the icon's inner SVG markup, without the `<svg>` element
/// itself. Render it with the [`icon`](../topcoat/icon/struct.icon.html) component.
#[derive(Debug, Clone)]
pub struct IconData {
    view_box: ViewBox,
    body: ViewHandle,
}

impl IconData {
    /// Creates an icon from its view box and body view.
    #[must_use]
    pub fn new(view_box: ViewBox, body: ViewHandle) -> Self {
        Self { view_box, body }
    }

    /// Creates an icon from trusted SVG markup. The body is rendered without
    /// escaping or validation, so it must not contain untrusted input.
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
