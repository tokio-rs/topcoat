use ref_cast::RefCast;
use serde::{Deserialize, Serialize};

use crate::runtime::{
    OptionSurrogate, Surrogate, impl_surrogate, impl_surrogate_mut, impl_surrogate_ref,
};

#[derive(Debug, RefCast, Clone, Copy, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct BoolSurrogate(bool);

impl BoolSurrogate {
    #[inline]
    pub(crate) const fn new(v: bool) -> Self {
        Self(v)
    }
}

impl_surrogate!(bool, BoolSurrogate);
impl_surrogate_ref!(bool, BoolSurrogate);
impl_surrogate_mut!(bool, BoolSurrogate);

impl core::ops::Not for BoolSurrogate {
    type Output = BoolSurrogate;

    #[inline]
    fn not(self) -> BoolSurrogate {
        BoolSurrogate(!self.0)
    }
}

macro_rules! impl_cmp_op {
    ($method:ident, $op:tt) => {
        impl BoolSurrogate {
            #[inline]
            #[must_use]
            pub fn $method(&self, rhs: &BoolSurrogate) -> BoolSurrogate {
                BoolSurrogate::new(self.0 $op rhs.0)
            }
        }
    };
}

impl_cmp_op!(eq, ==);
impl_cmp_op!(ne, !=);

impl BoolSurrogate {
    #[inline]
    pub fn then<F, S>(self, f: F) -> OptionSurrogate<S::Real>
    where
        F: FnOnce() -> S,
        S: Surrogate,
    {
        OptionSurrogate::new(self.0.then(|| f().into_real()))
    }

    #[inline]
    pub fn then_some<S>(self, t: S) -> OptionSurrogate<S::Real>
    where
        S: Surrogate,
    {
        OptionSurrogate::new(self.0.then_some(t.into_real()))
    }
}

impl std::fmt::Display for BoolSurrogate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
