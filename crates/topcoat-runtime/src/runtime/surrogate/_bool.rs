use ref_cast::RefCast;

use crate::runtime::{
    Option, Surrogate, deserialize_tagged, impl_surrogate, impl_surrogate_mut, impl_surrogate_ref,
    serialize_tagged,
};

#[derive(Debug, RefCast, Clone, Copy)]
#[repr(transparent)]
pub struct Bool(bool);

impl Bool {
    #[inline]
    pub(crate) const fn new(v: bool) -> Self {
        Self(v)
    }
}

impl_surrogate!(bool, Bool);
impl_surrogate_ref!(bool, Bool);
impl_surrogate_mut!(bool, Bool);

impl serde::Serialize for Bool {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serialize_tagged(serializer, "bool", &self.0)
    }
}

impl<'de> serde::Deserialize<'de> for Bool {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserialize_tagged(deserializer, "bool").map(Self)
    }
}

impl core::ops::Not for Bool {
    type Output = Bool;

    #[inline]
    fn not(self) -> Bool {
        Bool(!self.0)
    }
}

macro_rules! impl_cmp_op {
    ($method:ident, $op:tt) => {
        impl Bool {
            #[inline]
            pub fn $method(&self, rhs: &Bool) -> Bool {
                Bool::new(self.0 $op rhs.0)
            }
        }
    };
}

impl_cmp_op!(eq, ==);
impl_cmp_op!(ne, !=);

impl Bool {
    #[inline]
    pub fn then<F, S>(self, f: F) -> Option<S::Real>
    where
        F: FnOnce() -> S,
        S: Surrogate,
    {
        Option::new(self.0.then(|| f().into_real()))
    }

    #[inline]
    pub fn then_some<S>(self, t: S) -> Option<S::Real>
    where
        S: Surrogate,
    {
        Option::new(self.0.then_some(t.into_real()))
    }
}

impl std::fmt::Display for Bool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
