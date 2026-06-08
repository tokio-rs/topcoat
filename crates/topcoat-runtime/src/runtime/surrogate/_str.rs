use ref_cast::RefCast;

use crate::runtime::{Bool, F64, impl_surrogate_mut, impl_surrogate_ref, serialize_tagged};

#[derive(Debug, RefCast)]
#[repr(transparent)]
pub struct Str(pub(super) str);

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

impl Str {
    #[inline]
    pub fn is_empty(&self) -> Bool {
        Bool::new(self.0.is_empty())
    }

    #[inline]
    pub fn len(&self) -> F64 {
        F64::new(self.0.len() as f64)
    }

    #[inline]
    pub fn trim(&self) -> &Str {
        Str::ref_cast(self.0.trim())
    }

    #[inline]
    pub fn trim_start(&self) -> &Str {
        Str::ref_cast(self.0.trim_start())
    }

    #[inline]
    pub fn trim_end(&self) -> &Str {
        Str::ref_cast(self.0.trim_end())
    }

    #[inline]
    pub fn starts_with(&self, other: &Str) -> Bool {
        Bool::new(self.0.starts_with(&other.0))
    }

    #[inline]
    pub fn ends_with(&self, other: &Str) -> Bool {
        Bool::new(self.0.ends_with(&other.0))
    }

    #[inline]
    pub fn contains(&self, other: &Str) -> Bool {
        Bool::new(self.0.contains(&other.0))
    }
}
