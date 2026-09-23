use std::fmt::{self, Display};

use topcoat_core::context::Cx;

use crate::{AttributeValueViewParts, PartsWriter};

/// The [`viewBox`] attribute of an SVG element.
///
/// A view box can be used as an attribute value in a template, where it
/// renders as its four numbers separated by spaces, such as `0 0 24 24`. It
/// also implements [`Display`] with the same output.
///
/// [`viewBox`]: https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/viewBox
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewBox {
    /// The x coordinate of the top left corner.
    pub min_x: f32,
    /// The y coordinate of the top left corner.
    pub min_y: f32,
    /// The width of the visible area.
    pub width: f32,
    /// The height of the visible area.
    pub height: f32,
}

impl ViewBox {
    /// Creates a view box from its four components, in attribute order.
    #[must_use]
    pub const fn new(min_x: f32, min_y: f32, width: f32, height: f32) -> Self {
        Self {
            min_x,
            min_y,
            width,
            height,
        }
    }
}

impl Display for ViewBox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} {} {}",
            self.min_x, self.min_y, self.width, self.height
        )
    }
}

impl AttributeValueViewParts for ViewBox {
    fn attribute_present(&self) -> bool {
        true
    }

    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_f32(self.min_x);
        parts.push_promoted_str_unescaped(&" ");
        parts.push_f32(self.min_y);
        parts.push_promoted_str_unescaped(&" ");
        parts.push_f32(self.width);
        parts.push_promoted_str_unescaped(&" ");
        parts.push_f32(self.height);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::internal::Builder;

    fn render(value: impl AttributeValueViewParts) -> String {
        let cx = Cx::default();
        Builder::build(&cx, |b| b.attribute_value(value)).render(&cx)
    }

    #[test]
    fn displays_as_svg_view_box_value() {
        assert_eq!(ViewBox::new(0.0, 0.0, 24.0, 24.0).to_string(), "0 0 24 24");
        assert_eq!(
            ViewBox::new(0.0, -0.5, 16.5, 16.0).to_string(),
            "0 -0.5 16.5 16"
        );
    }

    #[test]
    fn renders_view_parts_as_space_separated_value() {
        assert_eq!(render(ViewBox::new(0.0, 0.0, 24.0, 24.0)), "0 0 24 24");
        assert_eq!(
            render(ViewBox::new(0.0, -0.5, 16.5, 16.0)),
            "0 -0.5 16.5 16"
        );
    }

    #[test]
    fn attribute_is_always_present() {
        assert!(ViewBox::new(0.0, 0.0, 24.0, 24.0).attribute_present());
    }
}
