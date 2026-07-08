use crate::runtime::{Unescaped, ViewPart, ViewParts};

/// Converts a value used as an element name into view parts.
///
/// When this trait is implemented on a type, it can be used in the element name position of an
/// element in the [`view!`](https://docs.rs/topcoat/latest/topcoat/view/macro.view.html) macro:
///
/// ```rust
/// # use topcoat::view::view;
/// # async fn example() -> topcoat::Result {
/// # let tag_name = "div";
/// view! {
///     <(tag_name)></(tag_name)>
/// }
/// # }
/// ```
pub trait ElementNameViewParts {
    /// Appends this element name to `parts`.
    fn into_view_parts(self, parts: &mut ViewParts);
}

macro_rules! impl_primitive {
    ($ty:ty) => {
        impl ElementNameViewParts for $ty {
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

impl ElementNameViewParts for &str {
    #[inline]
    fn into_view_parts(self, parts: &mut ViewParts) {
        let part: ViewPart = self.to_owned().into();
        parts.push(part);
    }
}

impl ElementNameViewParts for Unescaped<&str> {
    #[inline]
    fn into_view_parts(self, parts: &mut ViewParts) {
        let part: ViewPart = Unescaped::new_unchecked(String::from(*self)).into();
        parts.push(part);
    }
}

impl ElementNameViewParts for &String {
    #[inline]
    fn into_view_parts(self, parts: &mut ViewParts) {
        self.as_str().into_view_parts(parts);
    }
}

impl<'b, T: ?Sized> ElementNameViewParts for &&'b T
where
    &'b T: ElementNameViewParts,
{
    #[inline]
    fn into_view_parts(self, parts: &mut ViewParts) {
        (*self).into_view_parts(parts);
    }
}

macro_rules! impl_tuple {
    ($($ty:ident),+) => {
        impl<$($ty),+> ElementNameViewParts for ($($ty,)+)
        where
            $($ty: ElementNameViewParts,)+
        {
            #[inline]
            #[allow(non_snake_case)]
            fn into_view_parts(self, parts: &mut ViewParts) {
                let ($($ty,)+) = self;
                $($ty.into_view_parts(parts);)+
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
