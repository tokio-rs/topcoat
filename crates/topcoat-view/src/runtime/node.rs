use crate::runtime::{Unescaped, View, ViewPart, ViewParts};

/// Converts a value used in node position into view parts.
///
/// The `view!` macro uses this for dynamic child content. Implement it for
/// custom types that should be accepted where a node can appear.
pub trait NodeViewParts {
    /// Appends this value to `parts`.
    fn into_view_parts(self, parts: &mut ViewParts);
}

impl NodeViewParts for View {
    #[inline]
    fn into_view_parts(self, parts: &mut ViewParts) {
        parts.push(self);
    }
}

macro_rules! impl_primitive {
    ($ty:ty) => {
        impl NodeViewParts for $ty {
            #[inline]
            fn into_view_parts(self, parts: &mut ViewParts) {
                parts.push(self);
            }
        }
    };
    ($ty:ty, ref) => {
        impl_primitive!($ty);

        impl NodeViewParts for &$ty {
            #[inline]
            fn into_view_parts(self, parts: &mut ViewParts) {
                parts.push(*self);
            }
        }
    };
}

impl_primitive!(ViewPart);
impl_primitive!(bool, ref);
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

impl NodeViewParts for &String {
    #[inline]
    fn into_view_parts(self, parts: &mut ViewParts) {
        self.as_str().into_view_parts(parts);
    }
}

impl NodeViewParts for &str {
    #[inline]
    fn into_view_parts(self, parts: &mut ViewParts) {
        let part: ViewPart = self.to_owned().into();
        parts.push(part);
    }
}

impl NodeViewParts for Unescaped<&str> {
    #[inline]
    fn into_view_parts(self, parts: &mut ViewParts) {
        let part: ViewPart = Unescaped::new_unchecked(String::from(*self)).into();
        parts.push(part);
    }
}

impl<T> NodeViewParts for Option<T>
where
    T: NodeViewParts,
{
    #[inline]
    fn into_view_parts(self, parts: &mut ViewParts) {
        if let Some(value) = self {
            value.into_view_parts(parts);
        }
    }
}

impl<T> NodeViewParts for Vec<T>
where
    T: NodeViewParts,
{
    #[inline]
    fn into_view_parts(self, parts: &mut ViewParts) {
        for value in self {
            value.into_view_parts(parts);
        }
    }
}
