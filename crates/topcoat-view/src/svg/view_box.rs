use std::fmt::{self, Display};

use topcoat_core::context::Cx;

use crate::{AttributeValueViewParts, PartsWriter};

/// The [`viewBox`] of an SVG element: `min-x`, `min-y`, `width`, and `height`.
///
/// [`viewBox`]: https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/viewBox
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewBox {
    pub min_x: f32,
    pub min_y: f32,
    pub width: f32,
    pub height: f32,
}

impl ViewBox {
    /// Creates a view box from its components.
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
        let mut buffer = zmij::Buffer::new();
        f.write_str(buffer.format(self.min_x))?;
        f.write_str(" ")?;
        f.write_str(buffer.format(self.min_y))?;
        f.write_str(" ")?;
        f.write_str(buffer.format(self.width))?;
        f.write_str(" ")?;
        f.write_str(buffer.format(self.height))?;
        Ok(())
    }
}

impl AttributeValueViewParts for ViewBox {
    fn attribute_present(&self) -> bool {
        true
    }

    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_f32(self.min_x);
        parts.push_str_unescaped(" ");
        parts.push_f32(self.min_y);
        parts.push_str_unescaped(" ");
        parts.push_f32(self.width);
        parts.push_str_unescaped(" ");
        parts.push_f32(self.height);
    }
}

#[cfg(test)]
mod tests {
    use topcoat_core::context::Cx;

    use super::*;
    use crate::{HtmlContext, View, ViewParts};

    fn render(value: impl AttributeValueViewParts) -> String {
        let mut parts = ViewParts::new();
        value.into_view_parts(
            &Cx::default(),
            &mut PartsWriter::new(&mut parts, HtmlContext::AttributeValue),
        );
        View::new(parts).render(&Cx::default())
    }

    #[test]
    fn displays_as_svg_view_box_value() {
        assert_eq!(
            ViewBox::new(0.0, 0.0, 24.0, 24.0).to_string(),
            "0.0 0.0 24.0 24.0"
        );
        assert_eq!(
            ViewBox::new(0.0, -0.5, 16.5, 16.0).to_string(),
            "0.0 -0.5 16.5 16.0"
        );
    }

    #[test]
    fn renders_view_parts_as_space_separated_value() {
        assert_eq!(
            render(ViewBox::new(0.0, 0.0, 24.0, 24.0)),
            "0.0 0.0 24.0 24.0"
        );
        assert_eq!(
            render(ViewBox::new(0.0, -0.5, 16.5, 16.0)),
            "0.0 -0.5 16.5 16.0"
        );
    }

    #[test]
    fn attribute_is_always_present() {
        assert!(ViewBox::new(0.0, 0.0, 24.0, 24.0).attribute_present());
    }
}
