use ref_cast::RefCast;
use serde::{Deserialize, Serialize};

use crate::runtime::{Str, impl_surrogate, impl_surrogate_mut, impl_surrogate_ref};

#[derive(Debug, Clone, RefCast, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct String(std::string::String);

impl String {
    #[inline]
    pub(crate) const fn new(v: std::string::String) -> Self {
        Self(v)
    }
}

impl_surrogate!(std::string::String, String);
impl_surrogate_ref!(std::string::String, String);
impl_surrogate_mut!(std::string::String, String);

impl std::fmt::Display for String {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::ops::Deref for String {
    type Target = Str;

    #[inline]
    fn deref(&self) -> &Str {
        Str::ref_cast(self.0.as_str())
    }
}
