use std::fmt::{self, Display};

use crate::runtime::{AttributeValueViewParts, Unescaped, ViewParts};

/// A CSS length.
///
/// Plain numbers convert to [`Length::Px`], so a `#[into]` component
/// parameter accepts `size: 24` as a 24-pixel length.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Length {
    /// CSS pixels (`px`).
    Px(f32),
    /// The element's font size (`em`).
    Em(f32),
    /// The root element's font size (`rem`).
    Rem(f32),
}

impl Display for Length {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Px(value) => write!(f, "{value}px"),
            Self::Em(value) => write!(f, "{value}em"),
            Self::Rem(value) => write!(f, "{value}rem"),
        }
    }
}

impl From<u16> for Length {
    fn from(value: u16) -> Self {
        Self::Px(f32::from(value))
    }
}

impl From<f32> for Length {
    fn from(value: f32) -> Self {
        Self::Px(value)
    }
}

impl AttributeValueViewParts for Length {
    fn attribute_present(&self) -> bool {
        true
    }

    fn into_view_parts(self, parts: &mut ViewParts) {
        match self {
            Self::Px(inner) => {
                parts.push(inner);
                parts.push(Unescaped::new_unchecked("px"));
            }
            Self::Em(inner) => {
                parts.push(inner);
                parts.push(Unescaped::new_unchecked("em"));
            }
            Self::Rem(inner) => {
                parts.push(inner);
                parts.push(Unescaped::new_unchecked("rem"));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn displays_with_unit() {
        assert_eq!(Length::Px(24.0).to_string(), "24px");
        assert_eq!(Length::Em(1.0).to_string(), "1em");
        assert_eq!(Length::Rem(1.5).to_string(), "1.5rem");
    }

    #[test]
    fn numbers_convert_to_pixels() {
        assert_eq!(Length::from(24u16), Length::Px(24.0));
        assert_eq!(Length::from(1.5f64), Length::Px(1.5));
    }
}
