mod attributes;
mod key;
mod value;

pub use attributes::*;
pub use key::*;
pub use value::*;

use crate::runtime::{Unescaped, ViewPart, ViewParts};

/// A single HTML attribute.
///
/// The value decides whether the attribute is present. For example, `None`
/// and `false` values omit the attribute.
#[derive(Debug, Clone)]
pub struct Attribute<K, V> {
    key: K,
    value: V,
}

impl<K, V> Attribute<K, V> {
    /// Creates an attribute from a key and value.
    #[inline]
    pub fn new(key: K, value: V) -> Self {
        Self { key, value }
    }
}

/// Converts one or more attributes into view parts.
///
/// When this trait is implemented on a type, it can be used in the attribute position of an element
/// in the [`view!`](https://docs.rs/topcoat/latest/topcoat/view/macro.view.html) macro:
///
/// ```rust
/// # use topcoat::view::{Attributes, view};
/// # async fn example() -> topcoat::Result {
/// # let my_value = Attributes::new();
/// view! {
///     <input (my_value)>
/// }
/// # }
/// ```
///
/// The emitted view parts must contain a leading space for each attribute to separate them from
/// the element name or preceding attributes.
pub trait AttributeViewParts {
    /// Appends zero or more attributes to `parts`.
    fn into_view_parts(self, parts: &mut ViewParts);
}

impl<K, V> AttributeViewParts for Attribute<K, V>
where
    K: AttributeKeyViewParts,
    V: AttributeValueViewParts,
{
    #[inline]
    fn into_view_parts(self, parts: &mut ViewParts) {
        if self.value.attribute_present() {
            parts.push(Unescaped::new_unchecked(" "));
            self.key.into_view_parts(parts);
            parts.push(Unescaped::new_unchecked("=\""));
            self.value.into_view_parts(parts);
            parts.push(Unescaped::new_unchecked("\""));
        }
    }
}

impl AttributeViewParts for ViewPart {
    #[inline]
    fn into_view_parts(self, parts: &mut ViewParts) {
        parts.push(self);
    }
}

impl<T> AttributeViewParts for Option<T>
where
    T: AttributeViewParts,
{
    #[inline]
    fn into_view_parts(self, parts: &mut ViewParts) {
        if let Some(value) = self {
            value.into_view_parts(parts);
        }
    }
}

impl<T> AttributeViewParts for Vec<T>
where
    T: AttributeViewParts,
{
    #[inline]
    fn into_view_parts(self, parts: &mut ViewParts) {
        for value in self {
            value.into_view_parts(parts);
        }
    }
}

impl<'b, T: ?Sized> AttributeViewParts for &&'b T
where
    &'b T: AttributeViewParts,
{
    #[inline]
    fn into_view_parts(self, parts: &mut ViewParts) {
        (*self).into_view_parts(parts);
    }
}
