use std::fmt::{self, Display};

use crate::runtime::{AttributeValueViewParts, Unescaped, ViewParts};

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

    fn into_view_parts(self, parts: &mut ViewParts) {
        const SPACE: Unescaped<&str> = Unescaped::new_unchecked(" ");
        parts.push(self.min_x);
        parts.push(SPACE);
        parts.push(self.min_y);
        parts.push(SPACE);
        parts.push(self.width);
        parts.push(SPACE);
        parts.push(self.height);
    }
}

#[cfg(test)]
mod tests {
    use topcoat_core::runtime::context::Cx;

    use super::*;
    use crate::runtime::{FmtHtml, Formatter, ViewPart};

    fn render(value: impl AttributeValueViewParts) -> String {
        let mut parts = ViewParts::new();
        value.into_view_parts(&mut parts);
        let part: ViewPart = parts.into();
        let mut buf = String::new();
        let mut f = Formatter::new(&mut buf);
        part.fmt_html(&Cx::default(), &mut f);
        buf
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
