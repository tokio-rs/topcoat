use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A tuple of call arguments encoded as a JSON array.
///
/// An empty argument list is `[]`, while a single unit argument is `[null]`.
/// Each argument keeps its own serialization, including surrogate values.
#[derive(Debug)]
pub struct Arguments<T>(pub T);

impl Serialize for Arguments<()> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        [(); 0].serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Arguments<()> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        <[(); 0]>::deserialize(deserializer)?;
        Ok(Self(()))
    }
}

macro_rules! impl_arguments {
    ($($t:ident),+ $(,)?) => {
        impl<$($t: Serialize),+> Serialize for Arguments<($($t,)+)> {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                self.0.serialize(serializer)
            }
        }

        impl<'de, $($t: Deserialize<'de>),+> Deserialize<'de> for Arguments<($($t,)+)> {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                <($($t,)+)>::deserialize(deserializer).map(Self)
            }
        }
    };
}

impl_arguments!(T1);
impl_arguments!(T1, T2);
impl_arguments!(T1, T2, T3);
impl_arguments!(T1, T2, T3, T4);
impl_arguments!(T1, T2, T3, T4, T5);
impl_arguments!(T1, T2, T3, T4, T5, T6);
impl_arguments!(T1, T2, T3, T4, T5, T6, T7);
impl_arguments!(T1, T2, T3, T4, T5, T6, T7, T8);
impl_arguments!(T1, T2, T3, T4, T5, T6, T7, T8, T9);
impl_arguments!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
impl_arguments!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
impl_arguments!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);

#[cfg(test)]
mod tests {
    use serde_json::{from_str, to_string};

    use super::*;
    use crate::{Surrogate, Surrogated};

    #[test]
    fn empty_arguments_and_one_unit_argument_have_distinct_encodings() {
        assert_eq!(to_string(&Arguments(())).unwrap(), "[]");
        assert_eq!(to_string(&Arguments(((),))).unwrap(), "[null]");
        from_str::<Arguments<()>>("[]").unwrap();
        from_str::<Arguments<((),)>>("[null]").unwrap();
        assert_eq!(to_string(&()).unwrap(), "null");
    }

    #[test]
    fn arguments_require_an_array_of_the_declared_length() {
        for input in ["null", "{}", "[null]", "[true]"] {
            assert!(from_str::<Arguments<()>>(input).is_err(), "{input}");
        }
        for input in ["null", "true", "[]", "[true, false]", "[1]"] {
            assert!(from_str::<Arguments<(bool,)>>(input).is_err(), "{input}");
        }
    }

    #[test]
    fn arguments_preserve_each_values_surrogate_encoding() {
        let value = (42u128, Some(String::from("hello")));
        let arguments = Arguments(value.clone().into_surrogate());
        let json = to_string(&arguments).unwrap();
        assert_eq!(
            json,
            r#"[{"t":"u128","bits":128,"v":"42"},{"t":"Option","v":"hello"}]"#,
        );
        let decoded: Arguments<<(u128, Option<String>) as Surrogated>::Surrogate> =
            from_str(&json).unwrap();
        assert_eq!(decoded.0.into_real(), value);
    }
}
