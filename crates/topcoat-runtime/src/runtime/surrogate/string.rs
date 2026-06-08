use ref_cast::RefCast;

use crate::runtime::{
    Bool, deserialize_tagged, impl_surrogate, impl_surrogate_mut, impl_surrogate_ref,
    serialize_tagged,
};

#[derive(Debug, Clone, RefCast)]
#[repr(transparent)]
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

impl serde::Serialize for String {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serialize_tagged(serializer, "String", &self.0)
    }
}

impl<'de> serde::Deserialize<'de> for String {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserialize_tagged(deserializer, "String").map(Self)
    }
}

impl std::fmt::Display for String {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

macro_rules! impl_cmp_op {
    ($method:ident, $op:tt) => {
        impl String {
            #[inline]
            pub fn $method(&self, rhs: &String) -> Bool {
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
