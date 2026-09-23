mod attributes;
mod collector;
mod key;
mod value;

pub use attributes::*;
pub(crate) use collector::*;
pub use key::*;
use topcoat_core::context::Cx;
pub use value::*;

use crate::{HtmlContext, PartsWriter};

/// A single HTML attribute made of a key and a value.
///
/// The value decides whether the attribute is rendered. For example, a
/// `None` or `false` value leaves out the attribute.
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

/// A value that renders as zero or more whole attributes in a template.
///
/// A type that implements this trait can be used in the attribute position
/// of the [`view!`](https://docs.rs/topcoat/latest/topcoat/view/macro.view.html) macro:
///
/// ```rust
/// # use topcoat::view::{Attributes, View, component, view};
/// # #[component]
/// # async fn example() -> topcoat::Result<impl View> {
/// # let my_value = Attributes::new();
/// Ok(view! {
///     <input (my_value)>
/// })
/// # }
/// ```
///
/// An implementation must push a leading space before each attribute, to
/// separate it from the element name or the previous attribute. Wrapping the
/// key and value in an [`Attribute`] takes care of this.
pub trait AttributeViewParts {
    /// Pushes zero or more attributes into `parts`.
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>);
}

impl<K, V> AttributeViewParts for Attribute<K, V>
where
    K: AttributeKeyViewParts,
    V: AttributeValueViewParts,
{
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        if self.value.attribute_present() {
            parts.push_promoted_str_unescaped(&" ");
            parts.in_context(HtmlContext::AttributeKey, |parts| {
                self.key.into_view_parts(cx, parts);
            });
            parts.push_promoted_str_unescaped(&"=\"");
            parts.in_context(HtmlContext::AttributeValue, |parts| {
                self.value.into_view_parts(cx, parts);
            });
            parts.push_promoted_str_unescaped(&"\"");
        }
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
    use crate::internal::Builder;

    fn render(attribute: impl AttributeViewParts) -> String {
        let cx = Cx::default();
        Builder::build(&cx, |b| b.attributes(attribute)).render(&cx)
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
