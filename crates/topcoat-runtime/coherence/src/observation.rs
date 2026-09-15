use serde::{Deserialize, Serialize};

/// A value compared independently of runtime serialization and rendering.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", deny_unknown_fields)]
pub enum Value {
    Unit,
    Bool(bool),
    F64(String),
    String(String),
    None,
    Some(Box<Value>),
    Ok(Box<Value>),
    Err(Box<Value>),
    Tuple(Vec<Value>),
}

/// An expression's value, language panic, or JavaScript compilation error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", deny_unknown_fields)]
pub enum Outcome {
    Return(Value),
    Panic(String),
    CompileError(String),
}

impl Outcome {
    /// Compares values exactly and panics by category, retaining their messages
    /// for diagnostics.
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
