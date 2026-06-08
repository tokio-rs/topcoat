use ref_cast::RefCast;

use crate::runtime::{
    Bool, deserialize_tagged, impl_surrogate, impl_surrogate_mut, impl_surrogate_ref,
    serialize_tagged,
};

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

impl serde::Serialize for F64 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serialize_tagged(serializer, "f64", &self.0)
    }
}

impl<'de> serde::Deserialize<'de> for F64 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserialize_tagged(deserializer, "f64").map(Self)
    }
}

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
