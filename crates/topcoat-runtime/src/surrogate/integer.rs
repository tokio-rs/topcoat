use ref_cast::RefCast;
use serde::{Deserialize, Serialize, de, ser::SerializeStruct};

use crate::{BoolSurrogate, impl_surrogate, impl_surrogate_mut, impl_surrogate_ref};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Integer {
    t: String,
    bits: u32,
    v: String,
}

macro_rules! integer_op {
    ($surrogate:ident, $trait:ident, $method:ident, $checked:ident) => {
        impl core::ops::$trait for $surrogate {
            type Output = Self;

            #[inline]
            #[track_caller]
            fn $method(self, rhs: Self) -> Self {
                Self(
                    self.0
                        .$checked(rhs.0)
                        .expect(concat!("invalid integer ", stringify!($method),)),
                )
            }
        }
    };
}

macro_rules! integer_cmp {
    ($surrogate:ident, $method:ident, $op:tt) => {
        impl $surrogate {
            #[inline]
            pub fn $method(&self, rhs: &Self) -> BoolSurrogate {
                BoolSurrogate::new(self.0 $op rhs.0)
            }
        }
    };
}

macro_rules! integer_surrogate {
    ($real:ident, $surrogate:ident) => {
        #[doc = concat!("A `", stringify!($real), "` in a runtime expression.")]
        ///
        /// Arithmetic panics on overflow or division by zero in every build profile.
        #[derive(Debug, Clone, Copy, RefCast)]
        #[repr(transparent)]
        pub struct $surrogate($real);

        impl $surrogate {
            #[inline]
            pub(crate) const fn new(value: $real) -> Self {
                Self(value)
            }
        }

        impl_surrogate!($real, $surrogate);
        impl_surrogate_ref!($real, $surrogate);
        impl_surrogate_mut!($real, $surrogate);

        impl std::fmt::Display for $surrogate {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }

        impl Serialize for $surrogate {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                let mut value = serializer.serialize_struct("Integer", 3)?;
                value.serialize_field("t", stringify!($real))?;
                value.serialize_field("bits", &$real::BITS)?;
                value.serialize_field("v", &self.0.to_string())?;
                value.end()
            }
        }

        impl<'de> Deserialize<'de> for $surrogate {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let value = Integer::deserialize(deserializer)?;
                if value.t != stringify!($real) {
                    return Err(de::Error::invalid_value(
                        de::Unexpected::Str(&value.t),
                        &stringify!($real),
                    ));
                }
                if value.bits != $real::BITS {
                    return Err(de::Error::custom("integer width does not match the server"));
                }
                let integer: $real = value.v.parse().map_err(de::Error::custom)?;
                if value.v != integer.to_string() {
                    return Err(de::Error::custom("expected a canonical decimal integer"));
                }
                Ok(Self(integer))
            }
        }

        integer_op!($surrogate, Add, add, checked_add);
        integer_op!($surrogate, Sub, sub, checked_sub);
        integer_op!($surrogate, Mul, mul, checked_mul);
        integer_op!($surrogate, Div, div, checked_div);
        integer_op!($surrogate, Rem, rem, checked_rem);

        integer_cmp!($surrogate, eq, ==);
        integer_cmp!($surrogate, ne, !=);
        integer_cmp!($surrogate, gt, >);
        integer_cmp!($surrogate, lt, <);
        integer_cmp!($surrogate, ge, >=);
        integer_cmp!($surrogate, le, <=);
    };
}

macro_rules! signed_integer_surrogate {
    ($real:ident, $surrogate:ident) => {
        integer_surrogate!($real, $surrogate);

        impl core::ops::Neg for $surrogate {
            type Output = Self;

            #[inline]
            #[track_caller]
            fn neg(self) -> Self {
                Self(self.0.checked_neg().expect("integer negation overflow"))
            }
        }
    };
}

integer_surrogate!(u8, U8Surrogate);
integer_surrogate!(u16, U16Surrogate);
integer_surrogate!(u32, U32Surrogate);
integer_surrogate!(u64, U64Surrogate);
integer_surrogate!(u128, U128Surrogate);
integer_surrogate!(usize, UsizeSurrogate);
signed_integer_surrogate!(i8, I8Surrogate);
signed_integer_surrogate!(i16, I16Surrogate);
signed_integer_surrogate!(i32, I32Surrogate);
signed_integer_surrogate!(i64, I64Surrogate);
signed_integer_surrogate!(i128, I128Surrogate);
signed_integer_surrogate!(isize, IsizeSurrogate);

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    macro_rules! integer_tests {
        ($real:ident, $surrogate:ident) => {
            #[test]
            fn $real() {
                for value in [$real::MIN, 0, 1, $real::MAX] {
                    let wire = serde_json::to_value($surrogate::new(value)).unwrap();
                    assert_eq!(wire, json!({
                        "t": stringify!($real), "bits": $real::BITS, "v": value.to_string(),
                    }));
                    let decoded: $surrogate = serde_json::from_value(wire).unwrap();
                    assert_eq!(decoded.0, value);
                    assert_eq!(decoded.to_string(), value.to_string());
                }
                for digits in ["", "-0", "+1", "01", "1.0", "1e3", " 1", "1 ", "0xff"] {
                    assert!(serde_json::from_value::<$surrogate>(json!({
                        "t": stringify!($real), "bits": $real::BITS, "v": digits,
                    })).is_err());
                }
                for wire in [
                    json!({ "t": "f64", "bits": $real::BITS, "v": "1" }),
                    json!({ "t": stringify!($real), "bits": $real::BITS / 2, "v": "1" }),
                    json!({ "t": stringify!($real), "bits": $real::BITS, "v": 1 }),
                    json!({ "t": stringify!($real), "v": "1" }),
                    json!({ "t": stringify!($real), "bits": $real::BITS, "v": "1", "extra": true }),
                    json!({ "t": stringify!($real), "bits": $real::BITS, "v": format!("{}0", $real::MAX) }),
                ] {
                    assert!(serde_json::from_value::<$surrogate>(wire).is_err());
                }
            }
        };
    }

    integer_tests!(u8, U8Surrogate);
    integer_tests!(u16, U16Surrogate);
    integer_tests!(u32, U32Surrogate);
    integer_tests!(u64, U64Surrogate);
    integer_tests!(u128, U128Surrogate);
    integer_tests!(usize, UsizeSurrogate);
    integer_tests!(i8, I8Surrogate);
    integer_tests!(i16, I16Surrogate);
    integer_tests!(i32, I32Surrogate);
    integer_tests!(i64, I64Surrogate);
    integer_tests!(i128, I128Surrogate);
    integer_tests!(isize, IsizeSurrogate);
}
