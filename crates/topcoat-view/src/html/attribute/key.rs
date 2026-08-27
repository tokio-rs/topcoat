use std::{
    borrow::Borrow,
    hash::{Hash, Hasher},
};

use topcoat_core::context::Cx;

use crate::{Captured, HtmlContext, PartsWriter, PromotedStr, StaticStr, Unescaped};

/// Converts a value used as an attribute key into view parts.
///
/// When this trait is implemented on a type, it can be used in the attribute key position of an
/// element in the [`view!`](https://docs.rs/topcoat/latest/topcoat/view/macro.view.html) macro:
///
/// ```rust
/// # use topcoat::view::{component, view};
/// # #[component]
/// # async fn example() -> topcoat::Result {
/// # let my_key = "data-state";
/// view! {
///     <div (my_key)="value"></div>
/// }
/// # }
/// ```
pub trait AttributeKeyViewParts {
    /// Appends this attribute key to the view being built.
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>);
}

impl AttributeKeyViewParts for String {
    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_string(self);
    }
}

impl AttributeKeyViewParts for &str {
    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_str(self);
    }
}

impl AttributeKeyViewParts for PromotedStr {
    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_promoted_str(self.0);
    }
}

impl AttributeKeyViewParts for StaticStr {
    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_static_str(self.0);
    }
}

impl AttributeKeyViewParts for Unescaped<String> {
    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_string_unescaped(self.0);
    }
}

impl AttributeKeyViewParts for Unescaped<&'static str> {
    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_static_str_unescaped(self.0);
    }
}

impl AttributeKeyViewParts for Unescaped<PromotedStr> {
    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_promoted_str_unescaped(self.0.0);
    }
}

impl AttributeKeyViewParts for Unescaped<StaticStr> {
    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_static_str_unescaped(self.0.0);
    }
}

impl AttributeKeyViewParts for &String {
    #[inline]
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        self.as_str().into_view_parts(cx, parts);
    }
}

impl<'b, T: ?Sized> AttributeKeyViewParts for &&'b T
where
    &'b T: AttributeKeyViewParts,
{
    #[inline]
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        (*self).into_view_parts(cx, parts);
    }
}

macro_rules! impl_tuple {
    ($($ty:ident),+) => {
        impl<$($ty),+> AttributeKeyViewParts for ($($ty,)+)
        where
            $($ty: AttributeKeyViewParts,)+
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

/// An attribute key captured from any [`AttributeKeyViewParts`] value.
///
/// Produced by the [`Attributes`](crate::Attributes) collection. A key that
/// pushes a single string is kept as that string, so it costs nothing
/// beyond the string itself; anything else is rendered into a `String` when
/// it is captured. A variant carries the context it was pushed with, so
/// validation happens when the key is finally written into a view. Keys
/// compare and hash by their text alone.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum AttributeKey {
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

impl AttributeKey {
    /// Returns the key's text.
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::PromotedStr { value, .. } => value,
            Self::StaticStr { value, .. } => value,
            Self::String { value, .. } => value,
        }
    }

    /// Writes the key into `parts` under the context it was captured with.
    #[inline]
    fn push_into(self, parts: &mut PartsWriter<'_>) {
        match self {
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

    /// Writes the key into `parts` under the context it was captured with,
    /// copying an owned string instead of moving it.
    #[inline]
    fn push_ref_into(&self, parts: &mut PartsWriter<'_>) {
        match self {
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

impl PartialEq for AttributeKey {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for AttributeKey {}

impl Hash for AttributeKey {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

impl Borrow<str> for AttributeKey {
    #[inline]
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl Captured for AttributeKey {
    #[inline]
    fn empty() -> Self {
        Self::StaticStr {
            value: "",
            context: HtmlContext::AttributeKey,
        }
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

impl AttributeKeyViewParts for AttributeKey {
    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        self.push_into(parts);
    }
}

impl AttributeKeyViewParts for &AttributeKey {
    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        self.push_ref_into(parts);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::{Attributes, buffer::ViewBuffer, internal::Builder};

    /// Captures `key` the way [`Attributes::insert`] does.
    fn capture(key: impl AttributeKeyViewParts) -> AttributeKey {
        let cx = Cx::default();
        let mut attrs = Attributes::new();
        attrs.insert(&cx, key, true);
        attrs.into_iter().next().unwrap().0
    }

    /// Writes a captured key in attribute key position and renders it.
    fn render(key: &AttributeKey) -> String {
        let cx = Cx::default();
        ViewBuffer::build(|parts| Builder::new(&cx, parts).attribute_key(key)).render(&cx)
    }

    #[test]
    fn a_single_promoted_string_is_kept_as_is() {
        let key = capture(PromotedStr(&"data-x"));
        assert!(matches!(
            key,
            AttributeKey::PromotedStr {
                context: HtmlContext::AttributeKey,
                ..
            }
        ));
        assert_eq!(key.as_str(), "data-x");
        assert_eq!(render(&key), "data-x");
    }

    #[test]
    fn a_single_borrowed_string_is_owned() {
        let key = capture("data-x");
        assert!(matches!(key, AttributeKey::String { .. }));
        assert_eq!(key.as_str(), "data-x");
    }

    #[test]
    fn multiple_parts_are_rendered() {
        let key = capture(("data-", "x"));
        assert!(matches!(
            key,
            AttributeKey::String {
                context: HtmlContext::Unescaped,
                ..
            }
        ));
        assert_eq!(key.as_str(), "data-x");
        assert_eq!(render(&key), "data-x");
    }

    #[test]
    #[should_panic(expected = "invalid attribute key")]
    fn an_invalid_key_panics_when_rendered() {
        render(&capture("on click"));
    }

    #[test]
    fn keys_compare_and_hash_by_text() {
        let promoted = capture(PromotedStr(&"class"));
        let owned = capture(String::from("class"));
        assert_eq!(promoted, owned);
        let keys: HashSet<AttributeKey> = [promoted, owned].into_iter().collect();
        assert_eq!(keys.len(), 1);
        assert!(keys.contains("class"));
    }
}
