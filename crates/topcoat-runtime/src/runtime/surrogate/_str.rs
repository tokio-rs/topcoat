use ref_cast::RefCast;

use crate::runtime::{impl_surrogate_mut, impl_surrogate_ref, serialize_tagged};

#[derive(Debug, RefCast)]
#[repr(transparent)]
pub struct Str(str);

impl_surrogate_ref!(str, Str);
impl_surrogate_mut!(str, Str);

impl serde::Serialize for Str {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serialize_tagged(serializer, "str", &self.0)
    }
}

impl std::fmt::Display for Str {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
