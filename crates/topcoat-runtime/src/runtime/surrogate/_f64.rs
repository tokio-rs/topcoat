use ref_cast::RefCast;
use serde::{Deserialize, Serialize};

use crate::runtime::{Bool, impl_surrogate, impl_surrogate_mut, impl_surrogate_ref};

#[derive(Debug, RefCast, Clone, Copy, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
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

macro_rules! impl_math_op {
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

impl_math_op!(Add, add, +);
impl_math_op!(Sub, sub, -);
impl_math_op!(Mul, mul, *);
impl_math_op!(Div, div, /);

impl core::ops::Neg for F64 {
    type Output = F64;

    #[inline]
    fn neg(self) -> F64 {
        F64(-self.0)
    }
}

macro_rules! impl_cmp_op {
    ($method:ident, $op:tt) => {
        impl F64 {
            #[inline]
            pub fn $method(&self, rhs: &F64) -> Bool {
                Bool::new(self.0 $op rhs.0)
            }
        }
    };
}

impl_cmp_op!(eq, ==);
impl_cmp_op!(ne, !=);
impl_cmp_op!(gt, >);
impl_cmp_op!(lt, <);
impl_cmp_op!(ge, >=);
impl_cmp_op!(le, <=);

impl std::fmt::Display for F64 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
