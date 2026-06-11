use ref_cast::RefCast;
use serde::{Deserialize, Serialize};

use crate::runtime::{BoolSurrogate, impl_surrogate, impl_surrogate_mut, impl_surrogate_ref};

#[derive(Debug, RefCast, Clone, Copy, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct F64Surrogate(f64);

impl F64Surrogate {
    #[inline]
    pub(crate) const fn new(v: f64) -> Self {
        Self(v)
    }
}

impl_surrogate!(f64, F64Surrogate);
impl_surrogate_ref!(f64, F64Surrogate);
impl_surrogate_mut!(f64, F64Surrogate);

macro_rules! impl_math_op {
    ($trait:ident, $method:ident, $op:tt) => {
        impl core::ops::$trait for F64Surrogate {
            type Output = F64Surrogate;

            #[inline]
            fn $method(self, rhs: F64Surrogate) -> F64Surrogate {
                F64Surrogate(self.0 $op rhs.0)
            }
        }
    };
}

impl_math_op!(Add, add, +);
impl_math_op!(Sub, sub, -);
impl_math_op!(Mul, mul, *);
impl_math_op!(Div, div, /);

impl core::ops::Neg for F64Surrogate {
    type Output = F64Surrogate;

    #[inline]
    fn neg(self) -> F64Surrogate {
        F64Surrogate(-self.0)
    }
}

macro_rules! impl_cmp_op {
    ($method:ident, $op:tt) => {
        impl F64Surrogate {
            #[inline]
            pub fn $method(&self, rhs: &F64Surrogate) -> BoolSurrogate {
                BoolSurrogate::new(self.0 $op rhs.0)
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

impl std::fmt::Display for F64Surrogate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
