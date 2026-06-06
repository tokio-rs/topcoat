use crate::runtime::{Unescaped, ViewPart, ViewParts};

/// Converts a value used as an attribute value into view parts.
///
/// Attribute values can also control presence. `false` and `None` omit the
/// whole attribute; other built-in values are present.
pub trait AttributeValueViewParts {
    /// Returns whether the containing attribute should be rendered.
    fn attribute_present(&self) -> bool;

    /// Appends this attribute value to `parts`.
    fn into_view_parts(self, parts: &mut ViewParts);
}

macro_rules! impl_primitive {
    ($ty:ty) => {
        impl AttributeValueViewParts for $ty {
            #[inline]
            fn attribute_present(&self) -> bool {
                true
            }

            #[inline]
            fn into_view_parts(self, parts: &mut ViewParts) {
                parts.push(self);
            }
        }
    };
    ($ty:ty, ref) => {
        impl_primitive!($ty);

        impl AttributeValueViewParts for &$ty {
            #[inline]
            fn attribute_present(&self) -> bool {
                (*self).attribute_present()
            }

            #[inline]
            fn into_view_parts(self, parts: &mut ViewParts) {
                (*self).into_view_parts(parts);
            }
        }
    };
}

impl_primitive!(char, ref);
impl_primitive!(i8, ref);
impl_primitive!(i16, ref);
impl_primitive!(i32, ref);
impl_primitive!(i64, ref);
impl_primitive!(i128, ref);
impl_primitive!(isize, ref);
impl_primitive!(u8, ref);
impl_primitive!(u16, ref);
impl_primitive!(u32, ref);
impl_primitive!(u64, ref);
impl_primitive!(u128, ref);
impl_primitive!(usize, ref);
impl_primitive!(f32, ref);
impl_primitive!(f64, ref);
impl_primitive!(String);
impl_primitive!(Unescaped<String>);

impl AttributeValueViewParts for &str {
    #[inline]
    fn attribute_present(&self) -> bool {
        true
    }

    #[inline]
    fn into_view_parts(self, parts: &mut ViewParts) {
        let part: ViewPart = self.to_owned().into();
        parts.push(part);
    }
}

impl AttributeValueViewParts for Unescaped<&str> {
    #[inline]
    fn attribute_present(&self) -> bool {
        true
    }

    #[inline]
    fn into_view_parts(self, parts: &mut ViewParts) {
        let part: ViewPart = Unescaped::new_unchecked(String::from(*self)).into();
        parts.push(part);
    }
}

impl AttributeValueViewParts for &String {
    #[inline]
    fn attribute_present(&self) -> bool {
        self.as_str().attribute_present()
    }

    #[inline]
    fn into_view_parts(self, parts: &mut ViewParts) {
        self.as_str().into_view_parts(parts);
    }
}

impl AttributeValueViewParts for bool {
    #[inline]
    fn attribute_present(&self) -> bool {
        *self
    }

    #[inline]
    fn into_view_parts(self, parts: &mut ViewParts) {
        parts.push(self);
    }
}

impl AttributeValueViewParts for &bool {
    #[inline]
    fn attribute_present(&self) -> bool {
        (*self).attribute_present()
    }

    #[inline]
    fn into_view_parts(self, parts: &mut ViewParts) {
        (*self).into_view_parts(parts)
    }
}

impl<T> AttributeValueViewParts for Option<T>
where
    T: AttributeValueViewParts,
{
    #[inline]
    fn attribute_present(&self) -> bool {
        self.is_some()
    }

    #[inline]
    fn into_view_parts(self, parts: &mut ViewParts) {
        if let Some(value) = self {
            value.into_view_parts(parts);
        }
    }
}

impl AttributeValueViewParts for ViewPart {
    fn attribute_present(&self) -> bool {
        match self {
            Self::Empty => false,
            Self::Bool(false) => false,
            Self::BoxSlice(inner) if inner.is_empty() => false,
            Self::Vec(inner) if inner.is_empty() => false,
            _ => true,
        }
    }

    #[inline]
    fn into_view_parts(self, parts: &mut ViewParts) {
        parts.push(self);
    }
}
