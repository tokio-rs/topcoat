use crate::runtime::{Unescaped, ViewPart, ViewParts};

/// Converts a value used as an attribute key into view parts.
///
/// Implement this for custom dynamic attribute-name values accepted by `view!`.
pub trait AttributeKeyViewParts {
    /// Appends this attribute key to `parts`.
    fn into_view_parts(self, parts: &mut ViewParts);
}

macro_rules! impl_primitive {
    ($ty:ty) => {
        impl AttributeKeyViewParts for $ty {
            #[inline]
            fn into_view_parts(self, parts: &mut ViewParts) {
                parts.push(self);
            }
        }
    };
}

impl_primitive!(ViewPart);
impl_primitive!(String);
impl_primitive!(Unescaped<String>);

impl<'a> AttributeKeyViewParts for &'a str {
    #[inline]
    fn into_view_parts(self, parts: &mut ViewParts) {
        let part: ViewPart = self.to_owned().into();
        parts.push(part);
    }
}

impl<'a> AttributeKeyViewParts for Unescaped<&'a str> {
    #[inline]
    fn into_view_parts(self, parts: &mut ViewParts) {
        let part: ViewPart = Unescaped::new_unchecked(String::from(*self)).into();
        parts.push(part);
    }
}
