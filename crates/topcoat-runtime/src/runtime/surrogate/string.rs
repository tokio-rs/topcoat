use ref_cast::RefCast;
use serde::{Deserialize, Serialize};

use crate::runtime::{StrSurrogate, impl_surrogate, impl_surrogate_mut, impl_surrogate_ref};

#[derive(Debug, Clone, RefCast, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct StringSurrogate(String);

impl StringSurrogate {
    #[inline]
    pub(crate) const fn new(v: String) -> Self {
        Self(v)
    }
}

impl_surrogate!(String, StringSurrogate);
impl_surrogate_ref!(String, StringSurrogate);
impl_surrogate_mut!(String, StringSurrogate);

impl std::fmt::Display for StringSurrogate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::ops::Deref for StringSurrogate {
    type Target = StrSurrogate;

    #[inline]
    fn deref(&self) -> &StrSurrogate {
        StrSurrogate::ref_cast(self.0.as_str())
    }
}
