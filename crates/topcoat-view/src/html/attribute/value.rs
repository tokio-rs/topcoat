use std::borrow::Cow;

use topcoat_core::context::Cx;

use crate::{
    Captured, ClassViewParts, HtmlContext, PartsWriter, PromotedStr, StaticStr, Unescaped,
};

/// Converts a value used as an attribute value into view parts.
///
/// When this trait is implemented on a type, it can be used in the attribute value position of an
/// element in the [`view!`](https://docs.rs/topcoat/latest/topcoat/view/macro.view.html) macro:
///
/// ```rust
/// # use topcoat::view::{component, view};
/// # #[component]
/// # async fn example() -> topcoat::Result {
/// # let my_value = "primary";
/// view! {
///     <div class=(my_value)></div>
/// }
/// # }
/// ```
///
/// For [boolean HTML attributes], a false value must be omitted from the markup entirely.
/// [`attribute_present`](Self::attribute_present) is the hook that makes that decision.
/// The built-in `bool` and `Option<T>` implementations use this so `false` and `None` omit the
/// whole attribute, while `true` renders the attribute with an empty value (`disabled=""`).
///
/// [boolean HTML attributes]: https://developer.mozilla.org/en-US/docs/Glossary/Boolean/HTML
pub trait AttributeValueViewParts {
    /// Returns whether the containing attribute should be rendered.
    ///
    /// For [boolean HTML attributes], a false value must be omitted from the markup entirely.
    ///
    /// [boolean HTML attributes]: https://developer.mozilla.org/en-US/docs/Glossary/Boolean/HTML
    fn attribute_present(&self) -> bool;

    /// Appends this attribute value to the view being built.
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>);
}

macro_rules! impl_primitive {
    ($ty:ty, $method:ident) => {
        impl AttributeValueViewParts for $ty {
            #[inline]
            fn attribute_present(&self) -> bool {
                true
            }

            #[inline]
            fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
                parts.$method(self);
            }
        }
    };
    ($ty:ty, $method:ident, ref) => {
        impl_primitive!($ty, $method);

        impl AttributeValueViewParts for &$ty {
            #[inline]
            fn attribute_present(&self) -> bool {
                (*self).attribute_present()
            }

            #[inline]
            fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
                (*self).into_view_parts(cx, parts);
            }
        }
    };
}

impl_primitive!(char, push_char, ref);
impl_primitive!(i8, push_i8, ref);
impl_primitive!(i16, push_i16, ref);
impl_primitive!(i32, push_i32, ref);
impl_primitive!(i64, push_i64, ref);
impl_primitive!(i128, push_i128, ref);
impl_primitive!(isize, push_isize, ref);
impl_primitive!(u8, push_u8, ref);
impl_primitive!(u16, push_u16, ref);
impl_primitive!(u32, push_u32, ref);
impl_primitive!(u64, push_u64, ref);
impl_primitive!(u128, push_u128, ref);
impl_primitive!(usize, push_usize, ref);
impl_primitive!(f32, push_f32, ref);
impl_primitive!(f64, push_f64, ref);
impl_primitive!(String, push_string);

impl AttributeValueViewParts for Cow<'static, str> {
    #[inline]
    fn attribute_present(&self) -> bool {
        true
    }

    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        match self {
            Cow::Borrowed(value) => parts.push_static_str(value),
            Cow::Owned(value) => parts.push_string(value),
        };
    }
}

impl AttributeValueViewParts for &str {
    #[inline]
    fn attribute_present(&self) -> bool {
        true
    }

    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_str(self);
    }
}

impl AttributeValueViewParts for PromotedStr {
    #[inline]
    fn attribute_present(&self) -> bool {
        true
    }

    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_promoted_str(self.0);
    }
}

impl AttributeValueViewParts for StaticStr {
    #[inline]
    fn attribute_present(&self) -> bool {
        true
    }

    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_static_str(self.0);
    }
}

impl AttributeValueViewParts for Unescaped<String> {
    #[inline]
    fn attribute_present(&self) -> bool {
        true
    }

    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_string_unescaped(self.0);
    }
}

impl AttributeValueViewParts for Unescaped<&'static str> {
    #[inline]
    fn attribute_present(&self) -> bool {
        true
    }

    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_static_str_unescaped(self.0);
    }
}

impl AttributeValueViewParts for Unescaped<PromotedStr> {
    #[inline]
    fn attribute_present(&self) -> bool {
        true
    }

    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_promoted_str_unescaped(self.0.0);
    }
}

impl AttributeValueViewParts for Unescaped<StaticStr> {
    #[inline]
    fn attribute_present(&self) -> bool {
        true
    }

    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_static_str_unescaped(self.0.0);
    }
}

impl AttributeValueViewParts for &String {
    #[inline]
    fn attribute_present(&self) -> bool {
        self.as_str().attribute_present()
    }

    #[inline]
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        AttributeValueViewParts::into_view_parts(self.as_str(), cx, parts);
    }
}

impl AttributeValueViewParts for bool {
    #[inline]
    fn attribute_present(&self) -> bool {
        *self
    }

    #[inline]
    fn into_view_parts(self, _cx: &Cx, _parts: &mut PartsWriter<'_>) {
        // A true value renders the attribute with an empty value
        // (`disabled=""`), matching the boolean HTML attribute convention.
    }
}

impl AttributeValueViewParts for &bool {
    #[inline]
    fn attribute_present(&self) -> bool {
        (*self).attribute_present()
    }

    #[inline]
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        (*self).into_view_parts(cx, parts);
    }
}

impl<'b, T: ?Sized> AttributeValueViewParts for &&'b T
where
    &'b T: AttributeValueViewParts,
{
    #[inline]
    fn attribute_present(&self) -> bool {
        (**self).attribute_present()
    }

    #[inline]
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        (*self).into_view_parts(cx, parts);
    }
}

impl<T> AttributeValueViewParts for Option<T>
where
    T: AttributeValueViewParts,
{
    #[inline]
    fn attribute_present(&self) -> bool {
        self.as_ref()
            .is_some_and(AttributeValueViewParts::attribute_present)
    }

    #[inline]
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        if let Some(value) = self {
            value.into_view_parts(cx, parts);
        }
    }
}

macro_rules! impl_tuple {
    ($($ty:ident),+) => {
        impl<$($ty),+> AttributeValueViewParts for ($($ty,)+)
        where
            $($ty: AttributeValueViewParts,)+
        {
            #[inline]
            #[allow(non_snake_case)]
            fn attribute_present(&self) -> bool {
                let ($($ty,)+) = self;
                $($ty.attribute_present())||+
            }

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

/// An attribute value captured from any [`AttributeValueViewParts`] value.
///
/// Produced by the [`Attributes`](crate::Attributes) collection. A value
/// that pushes a single string is kept as that string, so it costs nothing
/// beyond the string itself; anything else is rendered into a `String` when
/// it is captured. A string variant carries the context it was pushed with,
/// so escaping happens when the value is finally written into a view. Using
/// a captured value as an attribute value or a class list entry writes it
/// back exactly as it was captured.
#[non_exhaustive]
#[derive(Debug, Default, Clone)]
pub enum AttributeValue {
    /// Marks the attribute as not rendered.
    ///
    /// An absent value keeps its key in an [`Attributes`](crate::Attributes)
    /// collection, so it still replaces an earlier value when collections
    /// are merged, but the attribute is not rendered.
    #[default]
    Absent,
    /// A present value that renders nothing, like a `true` boolean
    /// attribute (`disabled=""`).
    Empty,
    /// A static string held by reference.
    PromotedStr {
        value: &'static &'static str,
        context: HtmlContext,
    },
    /// A static string.
    StaticStr {
        value: &'static str,
        context: HtmlContext,
    },
    /// An owned string.
    String { value: String, context: HtmlContext },
}

impl AttributeValue {
    /// Returns the value that marks its attribute as absent.
    #[inline]
    #[must_use]
    pub fn absent() -> Self {
        Self::Absent
    }

    /// Returns whether the attribute holding this value should be rendered.
    #[inline]
    #[must_use]
    pub fn is_present(&self) -> bool {
        !matches!(self, Self::Absent)
    }

    /// Writes the value into `parts` under the context it was captured
    /// with.
    #[inline]
    fn push_into(self, parts: &mut PartsWriter<'_>) {
        match self {
            Self::Absent | Self::Empty => {}
            Self::PromotedStr { value, context } => {
                parts.in_context(context, |parts| {
                    parts.push_promoted_str(value);
                });
            }
            Self::StaticStr { value, context } => {
                parts.in_context(context, |parts| {
                    parts.push_static_str(value);
                });
            }
            Self::String { value, context } => {
                parts.in_context(context, |parts| {
                    parts.push_string(value);
                });
            }
        }
    }

    /// Writes the value into `parts` under the context it was captured
    /// with, copying an owned string instead of moving it.
    #[inline]
    fn push_ref_into(&self, parts: &mut PartsWriter<'_>) {
        match self {
            Self::Absent | Self::Empty => {}
            Self::PromotedStr { value, context } => {
                parts.in_context(*context, |parts| {
                    parts.push_promoted_str(value);
                });
            }
            Self::StaticStr { value, context } => {
                parts.in_context(*context, |parts| {
                    parts.push_static_str(value);
                });
            }
            Self::String { value, context } => {
                parts.in_context(*context, |parts| {
                    parts.push_str(value);
                });
            }
        }
    }
}

impl AttributeValueViewParts for AttributeValue {
    #[inline]
    fn attribute_present(&self) -> bool {
        self.is_present()
    }

    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        self.push_into(parts);
    }
}

impl AttributeValueViewParts for &AttributeValue {
    #[inline]
    fn attribute_present(&self) -> bool {
        self.is_present()
    }

    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        self.push_ref_into(parts);
    }
}

/// A captured attribute value spliced in as a single class list entry, such
/// as one taken from an [`Attributes`](crate::Attributes) collection with
/// [`remove`](crate::Attributes::remove). An absent value is skipped.
impl ClassViewParts for AttributeValue {
    #[inline]
    fn is_present(&self) -> bool {
        AttributeValue::is_present(self)
    }

    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        self.push_into(parts);
    }
}

impl ClassViewParts for &AttributeValue {
    #[inline]
    fn is_present(&self) -> bool {
        AttributeValue::is_present(self)
    }

    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        self.push_ref_into(parts);
    }
}

impl Captured for AttributeValue {
    #[inline]
    fn empty() -> Self {
        Self::Empty
    }

    #[inline]
    fn promoted_str(value: &'static &'static str, context: HtmlContext) -> Self {
        Self::PromotedStr { value, context }
    }

    #[inline]
    fn static_str(value: &'static str, context: HtmlContext) -> Self {
        Self::StaticStr { value, context }
    }

    #[inline]
    fn string(value: String, context: HtmlContext) -> Self {
        Self::String { value, context }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Attributes, buffer::ViewBuffer, internal::Builder};

    /// Captures `value` the way [`Attributes::insert`] does.
    fn capture(value: impl AttributeValueViewParts) -> AttributeValue {
        let cx = Cx::default();
        let mut attrs = Attributes::new();
        attrs.insert(&cx, "x", value);
        attrs.remove("x").unwrap()
    }

    /// Writes a captured value in attribute value position and renders it.
    fn render(value: &AttributeValue) -> String {
        let cx = Cx::default();
        ViewBuffer::build(|parts| Builder::new(&cx, parts).attribute_value(value)).render(&cx)
    }

    #[test]
    fn a_single_promoted_string_is_kept_as_is() {
        let value = capture(PromotedStr(&"a\"b"));
        assert!(matches!(
            value,
            AttributeValue::PromotedStr {
                value: &"a\"b",
                context: HtmlContext::AttributeValue,
            }
        ));
        assert_eq!(render(&value), "a&quot;b");
    }

    #[test]
    fn a_single_static_string_is_kept_as_is() {
        let value = capture(StaticStr("a"));
        assert!(matches!(
            value,
            AttributeValue::StaticStr { value: "a", .. }
        ));
        assert_eq!(render(&value), "a");
    }

    #[test]
    fn a_single_borrowed_string_is_owned_but_not_escaped_yet() {
        let value = capture("a\"b");
        let AttributeValue::String { value, context } = &value else {
            panic!("expected an owned string");
        };
        assert_eq!(value, "a\"b");
        assert_eq!(*context, HtmlContext::AttributeValue);
    }

    #[test]
    fn an_unescaped_string_keeps_its_context() {
        let value = capture(Unescaped::new_unchecked(PromotedStr(&"a&quot;b")));
        assert!(matches!(
            value,
            AttributeValue::PromotedStr {
                context: HtmlContext::Unescaped,
                ..
            }
        ));
        assert_eq!(render(&value), "a&quot;b");
    }

    #[test]
    fn a_true_boolean_is_empty() {
        let value = capture(true);
        assert!(matches!(value, AttributeValue::Empty));
        assert!(value.is_present());
        assert_eq!(render(&value), "");
    }

    #[test]
    fn a_false_boolean_and_none_are_absent() {
        assert!(matches!(capture(false), AttributeValue::Absent));
        assert!(matches!(
            capture(Option::<&str>::None),
            AttributeValue::Absent
        ));
    }

    #[test]
    fn multiple_parts_are_rendered_and_escaped_once() {
        let value = capture(("a\"", 1, Unescaped::new_unchecked(StaticStr("&"))));
        let AttributeValue::String { value, context } = &value else {
            panic!("expected a rendered string");
        };
        assert_eq!(value, "a&quot;1&");
        assert_eq!(*context, HtmlContext::Unescaped);
    }

    #[test]
    fn a_primitive_is_rendered() {
        let value = capture(42);
        assert!(matches!(value, AttributeValue::String { .. }));
        assert_eq!(render(&value), "42");
    }

    #[test]
    fn a_rendered_value_is_not_escaped_again() {
        let value = capture(("a\"", "b"));
        assert_eq!(render(&value), "a&quot;b");
    }

    #[test]
    fn a_captured_value_captures_as_itself() {
        let value = capture(PromotedStr(&"a"));
        let again = capture(&value);
        assert!(matches!(
            again,
            AttributeValue::PromotedStr { value: &"a", .. }
        ));
    }
}
