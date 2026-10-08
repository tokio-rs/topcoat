//! Icons drawn by hand. Declare an `IconData` constant for each SVG icon you use.

use topcoat::{icon::IconData, view::svg::ViewBox};

/// A four-pointed star. `currentColor` makes it inherit the surrounding text color.
pub const SPARKLE: IconData = IconData::unescaped_unchecked(
    ViewBox::new(0.0, 0.0, 24.0, 24.0),
    r#"<path fill="currentColor" d="M12 2l2.4 7.6L22 12l-7.6 2.4L12 22l-2.4-7.6L2 12l7.6-2.4z"/>"#,
);
