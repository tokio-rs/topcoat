use ref_cast::RefCast;

use crate::runtime::{
    deserialize_tagged, impl_surrogate, impl_surrogate_mut, impl_surrogate_ref, serialize_tagged,
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

impl std::fmt::Display for Bool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
