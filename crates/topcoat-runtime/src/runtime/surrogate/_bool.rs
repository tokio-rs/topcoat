use ref_cast::RefCast;
use serde::{Deserialize, Serialize};

use crate::runtime::{Option, Surrogate, impl_surrogate, impl_surrogate_mut, impl_surrogate_ref};

#[derive(Debug, RefCast, Clone, Copy, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
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
