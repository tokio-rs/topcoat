use topcoat_core::context::Cx;

use crate::{PartsWriter, Unescaped};

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
        parts.push_str(self);
    }
}

impl AttributeKeyViewParts for &str {
    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_str(self.to_owned());
    }
}

impl AttributeKeyViewParts for Unescaped<String> {
    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_str_unescaped(self.0);
    }
}

impl AttributeKeyViewParts for Unescaped<&'static str> {
    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_str_unescaped(self.0);
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
