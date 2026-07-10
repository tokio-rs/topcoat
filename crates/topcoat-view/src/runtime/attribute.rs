mod attributes;
mod key;
mod value;

pub use attributes::*;
pub use key::*;
pub use value::*;

use topcoat_core::runtime::context::Cx;

use crate::runtime::{HtmlContext, PartsWriter, ViewPart};

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
/// # use topcoat::view::{Attributes, component, view};
/// # #[component]
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
    /// Appends zero or more attributes to the view being built.
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>);
}

impl<K, V> AttributeViewParts for Attribute<K, V>
where
    K: AttributeKeyViewParts,
    V: AttributeValueViewParts,
{
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        if self.value.attribute_present() {
            parts.push_part(ViewPart::unescaped(" "));
            self.key
                .into_view_parts(cx, &mut parts.with_context(HtmlContext::AttributeKey));
            parts.push_part(ViewPart::unescaped("=\""));
            self.value
                .into_view_parts(cx, &mut parts.with_context(HtmlContext::AttributeValue));
            parts.push_part(ViewPart::unescaped("\""));
        }
    }
}

impl AttributeViewParts for ViewPart {
    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_part(self);
    }
}

impl<T> AttributeViewParts for Option<T>
where
    T: AttributeViewParts,
{
    #[inline]
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        if let Some(value) = self {
            value.into_view_parts(cx, parts);
        }
    }
}

impl<T> AttributeViewParts for Vec<T>
where
    T: AttributeViewParts,
{
    #[inline]
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        for value in self {
            value.into_view_parts(cx, parts);
        }
    }
}

impl<'b, T: ?Sized> AttributeViewParts for &&'b T
where
    &'b T: AttributeViewParts,
{
    #[inline]
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        (*self).into_view_parts(cx, parts);
    }
}

macro_rules! impl_tuple {
    ($($ty:ident),+) => {
        impl<$($ty),+> AttributeViewParts for ($($ty,)+)
        where
            $($ty: AttributeViewParts,)+
        {
            #[inline]
            #[allow(non_snake_case)]
            fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
                let ($($ty,)+) = self;
                $($ty.into_view_parts(cx, parts);)+
            }
        }
    };
}

impl_tuple!(T1);
impl_tuple!(T1, T2);
impl_tuple!(T1, T2, T3);
impl_tuple!(T1, T2, T3, T4);
impl_tuple!(T1, T2, T3, T4, T5);
impl_tuple!(T1, T2, T3, T4, T5, T6);
impl_tuple!(T1, T2, T3, T4, T5, T6, T7);
impl_tuple!(T1, T2, T3, T4, T5, T6, T7, T8);
impl_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9);
impl_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
impl_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
impl_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{View, ViewParts};

    fn render(attribute: impl AttributeViewParts) -> String {
        let mut parts = ViewParts::new();
        attribute.into_view_parts(
            &Cx::default(),
            &mut PartsWriter::new(&mut parts, HtmlContext::AttributeValue),
        );
        View::new(parts).render(&Cx::default())
    }

    #[test]
    fn renders_key_and_escaped_value() {
        let rendered = render(Attribute::new("data-x", "a\"b<c"));
        assert_eq!(rendered, " data-x=\"a&quot;b<c\"");
    }

    #[test]
    fn omits_absent_value() {
        assert_eq!(render(Attribute::new("disabled", false)), "");
    }

    #[test]
    fn dynamic_key_is_validated() {
        let rendered = render(Attribute::new(String::from("data-x"), "y"));
        assert_eq!(rendered, " data-x=\"y\"");
    }

    #[test]
    #[should_panic(expected = "invalid attribute key")]
    fn dynamic_key_rejects_breakout() {
        render(Attribute::new(String::from("x onmouseover"), "y"));
    }
}
