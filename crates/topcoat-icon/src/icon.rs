use topcoat_view::runtime::{Unescaped, View, svg::ViewBox};

/// The renderable data of an SVG icon: its view box and its body markup.
///
/// The body is the icon's inner SVG markup, without the `<svg>` element
/// itself. The element is supplied by the renderer, which controls sizing,
/// accessibility attributes, and any extra attributes on the root.
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

    /// Creates an icon whose body renders verbatim.
    ///
    /// Because this constructor is `const`, the resulting icon can be stored
    /// in `const` and `static` items.
    #[must_use]
    pub const fn unescaped(view_box: ViewBox, body: Unescaped<&'static str>) -> Self {
        Self {
            view_box,
            body: View::unescaped(body),
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
