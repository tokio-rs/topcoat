use ref_cast::RefCast;
use topcoat_view::runtime::{Unescaped, ViewParts};

use crate::runtime::{JsViewParts, impl_surrogate, impl_surrogate_mut, impl_surrogate_ref};

#[derive(Debug, RefCast, Clone, Copy)]
#[repr(transparent)]
pub struct F64(f64);

impl F64 {
    #[inline]
    pub(crate) const fn new(v: f64) -> Self {
        Self(v)
    }
}

impl_surrogate!(f64, F64);
impl_surrogate_ref!(f64, F64);
impl_surrogate_mut!(f64, F64);

impl JsViewParts for F64 {
    fn to_view_parts(&self, parts: &mut ViewParts) {
        parts.push(Unescaped::new_unchecked("cx.s.f64("));
        parts.push(self.0);
        parts.push(Unescaped::new_unchecked(")"));
    }
}

macro_rules! impl_binary_op {
    ($trait:ident, $method:ident, $op:tt) => {
        impl core::ops::$trait for F64 {
            type Output = F64;

            #[inline]
            fn $method(self, rhs: F64) -> F64 {
                F64(self.0 $op rhs.0)
            }
        }
    };
}

impl_binary_op!(Add, add, +);
impl_binary_op!(Sub, sub, -);
impl_binary_op!(Mul, mul, *);
impl_binary_op!(Div, div, /);

impl std::fmt::Display for F64 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
