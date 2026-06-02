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
}

impl_primitive!(ViewPart);
impl_primitive!(bool);
impl_primitive!(char);
impl_primitive!(i8);
impl_primitive!(i16);
impl_primitive!(i32);
impl_primitive!(i64);
impl_primitive!(i128);
impl_primitive!(isize);
impl_primitive!(u8);
impl_primitive!(u16);
impl_primitive!(u32);
impl_primitive!(u64);
impl_primitive!(u128);
impl_primitive!(usize);
impl_primitive!(f32);
impl_primitive!(f64);
impl_primitive!(String);
impl_primitive!(Unescaped<String>);

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

impl<T> NodeViewParts for &T
where
    T: NodeViewParts + Copy,
{
    #[inline]
    fn into_view_parts(self, parts: &mut ViewParts) {
        (*self).into_view_parts(parts);
    }
}
