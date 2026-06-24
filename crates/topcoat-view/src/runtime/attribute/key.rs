use crate::runtime::{Unescaped, ViewPart, ViewParts};

/// Converts a value used as an attribute key into view parts.
///
/// When this trait is implemented on a type, it can be used in the attribute key position of an
/// element in the [`view!`](https://docs.rs/topcoat/latest/topcoat/view/macro.view.html) macro:
///
/// ```rust
/// # use topcoat::view::view;
/// # async fn example() -> topcoat::Result {
/// # let my_key = "data-state";
/// view! {
///     <div (my_key)="value"></div>
/// }
/// # }
/// ```
pub trait AttributeKeyViewParts {
    /// Appends this attribute key to `parts`.
    fn into_view_parts(self, parts: &mut ViewParts);
}

macro_rules! impl_primitive {
    ($ty:ty) => {
        impl AttributeKeyViewParts for $ty {
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

impl AttributeKeyViewParts for &str {
    #[inline]
    fn into_view_parts(self, parts: &mut ViewParts) {
        let part: ViewPart = self.to_owned().into();
        parts.push(part);
    }
}

impl AttributeKeyViewParts for Unescaped<&str> {
    #[inline]
    fn into_view_parts(self, parts: &mut ViewParts) {
        let part: ViewPart = Unescaped::new_unchecked(String::from(*self)).into();
        parts.push(part);
    }
}

impl AttributeKeyViewParts for &String {
    #[inline]
    fn into_view_parts(self, parts: &mut ViewParts) {
        self.as_str().into_view_parts(parts);
    }
}

impl<'b, T: ?Sized> AttributeKeyViewParts for &&'b T
where
    &'b T: AttributeKeyViewParts,
{
    #[inline]
    fn into_view_parts(self, parts: &mut ViewParts) {
        (*self).into_view_parts(parts);
    }
}
