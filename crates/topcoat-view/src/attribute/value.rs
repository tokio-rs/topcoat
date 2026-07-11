use topcoat_core::context::Cx;

use crate::{PartsWriter, Unescaped, ViewPart};

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
/// whole attribute.
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
impl_primitive!(String, push_str);

impl AttributeValueViewParts for &str {
    #[inline]
    fn attribute_present(&self) -> bool {
        true
    }

    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_str(self.to_owned());
    }
}

impl AttributeValueViewParts for Unescaped<String> {
    #[inline]
    fn attribute_present(&self) -> bool {
        true
    }

    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_str_unescaped(self.0);
    }
}

impl AttributeValueViewParts for Unescaped<&'static str> {
    #[inline]
    fn attribute_present(&self) -> bool {
        true
    }

    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_str_unescaped(self.0);
    }
}

impl AttributeValueViewParts for &String {
    #[inline]
    fn attribute_present(&self) -> bool {
        self.as_str().attribute_present()
    }

    #[inline]
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        self.as_str().into_view_parts(cx, parts);
    }
}

impl AttributeValueViewParts for bool {
    #[inline]
    fn attribute_present(&self) -> bool {
        *self
    }

    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_bool(self);
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

impl AttributeValueViewParts for ViewPart {
    fn attribute_present(&self) -> bool {
        match self {
            Self::Empty | Self::Bool(false) => false,
            Self::BoxSlice { inner, .. } if inner.is_empty() => false,
            Self::Vec { inner, .. } if inner.is_empty() => false,
            _ => true,
        }
    }

    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_part(self);
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
