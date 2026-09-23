use ref_cast::RefCast;

use crate::{
    BoolSurrogate, F64Surrogate, StringSurrogate, impl_surrogate_mut, impl_surrogate_ref,
    serialize_tagged,
};

/// A `str` in a runtime expression.
///
/// Its methods follow Rust's behavior in the browser too: lengths count
/// UTF-8 bytes, comparisons order by code point, and trimming uses the
/// Unicode `White_Space` property.
#[derive(Debug, RefCast)]
#[repr(transparent)]
pub struct StrSurrogate(pub(super) str);

impl_surrogate_ref!(str, StrSurrogate);
impl_surrogate_mut!(str, StrSurrogate);

impl serde::Serialize for StrSurrogate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serialize_tagged(serializer, "str", &self.0)
    }
}

impl std::fmt::Display for StrSurrogate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

macro_rules! impl_cmp_op {
    ($method:ident, $op:tt) => {
        impl StrSurrogate {
            #[doc = concat!("Compares two strings with `", stringify!($op), "`.")]
            #[inline]
            pub fn $method(&self, rhs: &StrSurrogate) -> BoolSurrogate {
                BoolSurrogate::new(self.0 $op rhs.0)
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

impl StrSurrogate {
    /// Copies the string into an owned `String`.
    #[inline]
    #[must_use]
    pub fn to_owned(&self) -> StringSurrogate {
        StringSurrogate::new(self.0.to_owned())
    }

    /// Returns whether the string has a length of zero bytes.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> BoolSurrogate {
        BoolSurrogate::new(self.0.is_empty())
    }

    /// Returns the length of the string in UTF-8 bytes, as an `f64`.
    #[inline]
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn len(&self) -> F64Surrogate {
        F64Surrogate::new(self.0.len() as f64)
    }

    /// Returns the string without leading and trailing whitespace.
    #[inline]
    #[must_use]
    pub fn trim(&self) -> &StrSurrogate {
        StrSurrogate::ref_cast(self.0.trim())
    }

    /// Returns the string without leading whitespace.
    #[inline]
    #[must_use]
    pub fn trim_start(&self) -> &StrSurrogate {
        StrSurrogate::ref_cast(self.0.trim_start())
    }

    /// Returns the string without trailing whitespace.
    #[inline]
    #[must_use]
    pub fn trim_end(&self) -> &StrSurrogate {
        StrSurrogate::ref_cast(self.0.trim_end())
    }

    /// Returns whether the string starts with `other`.
    #[inline]
    #[must_use]
    pub fn starts_with(&self, other: &StrSurrogate) -> BoolSurrogate {
        BoolSurrogate::new(self.0.starts_with(&other.0))
    }

    /// Returns whether the string ends with `other`.
    #[inline]
    #[must_use]
    pub fn ends_with(&self, other: &StrSurrogate) -> BoolSurrogate {
        BoolSurrogate::new(self.0.ends_with(&other.0))
    }

    /// Returns whether `other` occurs in the string.
    #[inline]
    #[must_use]
    pub fn contains(&self, other: &StrSurrogate) -> BoolSurrogate {
        BoolSurrogate::new(self.0.contains(&other.0))
    }
}
