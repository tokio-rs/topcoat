use ref_cast::RefCast;

use crate::runtime::{Surrogated, impl_surrogate, impl_surrogate_mut, impl_surrogate_ref};

#[derive(RefCast, Clone, Copy)]
#[repr(transparent)]
#[allow(non_camel_case_types)]
pub struct f64(core::primitive::f64);

impl f64 {
    #[inline]
    pub(crate) const fn new(v: core::primitive::f64) -> Self {
        Self(v)
    }
}

impl_surrogate!(core::primitive::f64, f64);
impl_surrogate_ref!(core::primitive::f64, f64);
impl_surrogate_mut!(core::primitive::f64, f64);

macro_rules! impl_binary_op {
    ($trait:ident, $method:ident, $op:tt) => {
        impl core::ops::$trait for f64 {
            type Output = f64;

            #[inline]
            fn $method(self, rhs: f64) -> f64 {
                f64(self.0 $op rhs.0)
            }
        }
    };
}

impl_binary_op!(Add, add, +);
impl_binary_op!(Sub, sub, -);
impl_binary_op!(Mul, mul, *);
impl_binary_op!(Div, div, /);
