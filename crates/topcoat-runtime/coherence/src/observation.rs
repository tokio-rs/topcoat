use serde::{Deserialize, Serialize};

/// A value in the form both sides report it for comparison, independent of
/// the runtime's own serialization and rendering.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", deny_unknown_fields)]
pub enum Value {
    /// The unit value `()`.
    Unit,
    /// A `bool`.
    Bool(bool),
    /// An `f64` as its bits in hex, or `"nan"` for every NaN.
    F64(String),
    /// An integer with its Rust type name, bit width, and decimal digits.
    Integer {
        /// The Rust type name, such as `"u8"`.
        kind: String,
        /// The width in bits.
        bits: u32,
        /// The value in decimal.
        digits: String,
    },
    /// A string.
    String(String),
    /// `Option::None`.
    None,
    /// `Option::Some` with its value.
    Some(Box<Value>),
    /// `Result::Ok` with its value.
    Ok(Box<Value>),
    /// `Result::Err` with its value.
    Err(Box<Value>),
    /// A tuple with its fields in order.
    Tuple(Vec<Value>),
    /// A vector, array, or slice with its elements in order.
    Sequence(Vec<Value>),
}

/// How an evaluation ended: with a value, a panic, or a JavaScript error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", deny_unknown_fields)]
pub enum Outcome {
    /// The expression returned this value.
    Return(Value),
    /// The expression panicked in Rust or threw the runtime's `Panic` in
    /// JavaScript, with this message.
    Panic(String),
    /// The generated JavaScript failed to compile, with this error.
    CompileError(String),
    /// The JavaScript threw an exception other than a `Panic`.
    Exception(String),
}

impl Outcome {
    /// Whether two outcomes count as the same.
    ///
    /// Returned values must be equal. Two panics always agree, whatever their
    /// messages. Every other combination disagrees.
    #[must_use]
    pub fn agrees_with(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Return(left), Self::Return(right)) => left == right,
            (Self::Panic(_), Self::Panic(_)) => true,
            _ => false,
        }
    }
}

/// Converts a Rust value into the harness's comparison format.
pub trait Observe {
    /// Returns this value as a [`Value`].
    fn observe(&self) -> Value;
}

impl Observe for () {
    fn observe(&self) -> Value {
        Value::Unit
    }
}

impl Observe for bool {
    fn observe(&self) -> Value {
        Value::Bool(*self)
    }
}

impl Observe for f64 {
    fn observe(&self) -> Value {
        Value::F64(if self.is_nan() {
            "nan".to_owned()
        } else {
            format!("{:016x}", self.to_bits())
        })
    }
}

macro_rules! observe_integer {
    ($($integer:ident),+ $(,)?) => {
        $(impl Observe for $integer {
            fn observe(&self) -> Value {
                Value::Integer {
                    kind: stringify!($integer).to_owned(),
                    bits: $integer::BITS,
                    digits: self.to_string(),
                }
            }
        })+
    };
}

observe_integer!(
    u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize
);

impl Observe for str {
    fn observe(&self) -> Value {
        Value::String(self.to_owned())
    }
}

impl Observe for String {
    fn observe(&self) -> Value {
        self.as_str().observe()
    }
}

impl<T: Observe + ?Sized> Observe for &T {
    fn observe(&self) -> Value {
        (**self).observe()
    }
}

impl<T: Observe> Observe for [T] {
    fn observe(&self) -> Value {
        Value::Sequence(self.iter().map(Observe::observe).collect())
    }
}

impl<T: Observe> Observe for Vec<T> {
    fn observe(&self) -> Value {
        self.as_slice().observe()
    }
}

impl<T: Observe, const N: usize> Observe for [T; N] {
    fn observe(&self) -> Value {
        self.as_slice().observe()
    }
}

impl<T: Observe> Observe for Option<T> {
    fn observe(&self) -> Value {
        match self {
            None => Value::None,
            Some(value) => Value::Some(Box::new(value.observe())),
        }
    }
}

impl<T: Observe, E: Observe> Observe for Result<T, E> {
    fn observe(&self) -> Value {
        match self {
            Ok(value) => Value::Ok(Box::new(value.observe())),
            Err(value) => Value::Err(Box::new(value.observe())),
        }
    }
}

macro_rules! observe_tuple {
    ($($ty:ident $index:tt),+) => {
        impl<$($ty: Observe),+> Observe for ($($ty,)+) {
            fn observe(&self) -> Value {
                Value::Tuple(vec![$(self.$index.observe()),+])
            }
        }
    };
}

observe_tuple!(A 0);
observe_tuple!(A 0, B 1);
observe_tuple!(A 0, B 1, C 2);
observe_tuple!(A 0, B 1, C 2, D 3);
observe_tuple!(A 0, B 1, C 2, D 3, E 4);
observe_tuple!(A 0, B 1, C 2, D 3, E 4, F 5);
observe_tuple!(A 0, B 1, C 2, D 3, E 4, F 5, G 6);
observe_tuple!(A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7);
observe_tuple!(A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8);
observe_tuple!(A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9);
observe_tuple!(A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9, K 10);
observe_tuple!(A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9, K 10, L 11);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_float_distinctions_except_nan_payloads() {
        assert_ne!(0.0.observe(), (-0.0).observe());
        assert_ne!(f64::INFINITY.observe(), f64::NEG_INFINITY.observe());
        assert_eq!(
            f64::NAN.observe(),
            f64::from_bits(0x7ff8_0000_0000_0001).observe()
        );
    }

    #[test]
    fn preserves_variants_and_unit() {
        assert_ne!(Some(()).observe(), None::<()>.observe());
        assert_ne!(Ok::<_, ()>(()).observe(), Err::<(), _>(()).observe());
        assert_ne!((true,).observe(), true.observe());
    }
}
