use ref_cast::RefCast;

use crate::runtime::{Bool, impl_surrogate_mut, impl_surrogate_ref, serialize_tagged};

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

macro_rules! impl_cmp_op {
    ($method:ident, $op:tt) => {
        impl Str {
            #[inline]
            pub fn $method(&self, rhs: &Str) -> Bool {
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
