use ref_cast::RefCast;

use crate::runtime::{
    deserialize_tagged, impl_surrogate, impl_surrogate_mut, impl_surrogate_ref, serialize_tagged,
};

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

impl serde::Serialize for I32 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serialize_tagged(serializer, "i32", &self.0)
    }
}

impl<'de> serde::Deserialize<'de> for I32 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserialize_tagged(deserializer, "i32").map(Self)
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
