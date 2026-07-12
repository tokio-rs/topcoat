use topcoat_view::{View, svg::ViewBox};

/// The renderable data of an SVG icon: its view box and its body markup.
///
/// The body is the icon's inner SVG markup, without the `<svg>` element
/// itself. Pass it to the [`icon`](../topcoat/icon/struct.icon.html) component
/// to turn it into a renderable HTML element.
#[derive(Debug, Clone)]
pub struct IconData {
    view_box: ViewBox,
    body: View,
}

impl IconData {
    /// Creates an icon from its view box and body view.
    #[must_use]
    pub fn new(view_box: ViewBox, body: View) -> Self {
        Self { view_box, body }
    }

    /// Creates an icon whose body renders verbatim. The body is not checked
    /// for syntax errors or XSS injections.
    #[must_use]
    pub const fn unescaped_unchecked(view_box: ViewBox, body: &'static str) -> Self {
        Self {
            view_box,
            body: View::unescaped_unchecked(body),
        }
    }

    /// The icon's view box.
    #[must_use]
    pub const fn view_box(&self) -> ViewBox {
        self.view_box
    }

    /// Consumes the icon and returns its body view.
    #[must_use]
    pub fn into_body(self) -> View {
        self.body
    }
}
