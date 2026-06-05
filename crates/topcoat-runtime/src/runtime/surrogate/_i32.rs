use ref_cast::RefCast;
use topcoat_view::runtime::{Unescaped, ViewParts};

use crate::runtime::{JsViewParts, impl_surrogate, impl_surrogate_mut, impl_surrogate_ref};

#[derive(Debug, RefCast, Clone, Copy)]
#[repr(transparent)]
pub struct I32(i32);

impl I32 {
    #[inline]
    pub(crate) const fn new(v: i32) -> Self {
        Self(v)
    }
}

impl_surrogate!(i32, I32);
impl_surrogate_ref!(i32, I32);
impl_surrogate_mut!(i32, I32);

impl JsViewParts for I32 {
    fn to_view_parts(&self, parts: &mut ViewParts) {
        parts.push(Unescaped::new_unchecked("cx.s.i32("));
        parts.push(self.0);
        parts.push(Unescaped::new_unchecked(")"));
    }
}

macro_rules! impl_binary_op {
    ($trait:ident, $method:ident, $op:tt) => {
        impl core::ops::$trait for I32 {
            type Output = I32;

            #[inline]
            fn $method(self, rhs: I32) -> I32 {
                I32(self.0 $op rhs.0)
            }
        }
    };
}

impl_binary_op!(Add, add, +);
impl_binary_op!(Sub, sub, -);
impl_binary_op!(Mul, mul, *);
impl_binary_op!(Div, div, /);

impl std::fmt::Display for I32 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
