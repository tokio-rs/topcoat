use std::borrow::Cow;

use form_urlencoded::Parse;
use serde::{
    Deserialize,
    de::{self, IntoDeserializer, value::MapDeserializer},
    forward_to_deserialize_any,
};

type Error = serde_urlencoded::de::Error;

macro_rules! forward_parsed_value {
    ($($ty:ident => $method:ident),* $(,)?) => {
        $(
            fn $method<V>(self, visitor: V) -> Result<V::Value, Self::Error>
            where
                V: de::Visitor<'de>,
            {
                self.value
                    .parse::<$ty>()
                    .map_err(de::Error::custom)?
                    .into_deserializer()
                    .$method(visitor)
            }
        )*
    };
}

pub(crate) fn from_bytes<'de, T>(bytes: &'de [u8]) -> Result<T, serde_path_to_error::Error<Error>>
where
    T: Deserialize<'de>,
{
    serde_path_to_error::deserialize(Deserializer::new(form_urlencoded::parse(bytes)))
}

struct Deserializer<'de> {
    inner: MapDeserializer<'de, PartIterator<'de>, Error>,
}

impl<'de> Deserializer<'de> {
    fn new(parser: Parse<'de>) -> Self {
        Self {
            inner: MapDeserializer::new(PartIterator(parser)),
        }
    }
}

impl<'de> de::Deserializer<'de> for Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_map(self.inner)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_seq(self.inner)
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.inner.end()?;
        visitor.visit_unit()
    }

    forward_to_deserialize_any! {
        bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char str string option bytes byte_buf
        unit_struct newtype_struct tuple_struct struct identifier tuple enum ignored_any
    }
}

struct PartIterator<'de>(Parse<'de>);

impl<'de> Iterator for PartIterator<'de> {
    type Item = (Part<'de>, Part<'de>);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(key, value)| {
            (
                Part {
                    value: key,
                    empty_as_none: false,
                },
                Part {
                    value,
                    empty_as_none: true,
                },
            )
        })
    }
}

struct Part<'de> {
    value: Cow<'de, str>,
    empty_as_none: bool,
}

impl<'de> IntoDeserializer<'de> for Part<'de> {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

impl<'de> de::Deserializer<'de> for Part<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.value {
            Cow::Borrowed(value) => visitor.visit_borrowed_str(value),
            Cow::Owned(value) => visitor.visit_string(value),
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        if self.empty_as_none && self.value.is_empty() {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_enum(EnumAccess(self.value))
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    forward_to_deserialize_any! {
        char str string unit bytes byte_buf unit_struct tuple_struct struct identifier tuple
        ignored_any seq map
    }

    forward_parsed_value! {
        bool => deserialize_bool,
        u8 => deserialize_u8,
        u16 => deserialize_u16,
        u32 => deserialize_u32,
        u64 => deserialize_u64,
        i8 => deserialize_i8,
        i16 => deserialize_i16,
        i32 => deserialize_i32,
        i64 => deserialize_i64,
        f32 => deserialize_f32,
        f64 => deserialize_f64,
    }
}

struct EnumAccess<'de>(Cow<'de, str>);

impl<'de> de::EnumAccess<'de> for EnumAccess<'de> {
    type Error = Error;
    type Variant = UnitVariantAccess;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        let variant = seed.deserialize(self.0.into_deserializer())?;
        Ok((variant, UnitVariantAccess))
    }
}

struct UnitVariantAccess;

impl<'de> de::VariantAccess<'de> for UnitVariantAccess {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, _seed: T) -> Result<T::Value, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        Err(de::Error::custom("expected unit variant"))
    }

    fn tuple_variant<V>(self, _len: usize, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(de::Error::custom("expected unit variant"))
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(de::Error::custom("expected unit variant"))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde::Deserialize;

    use super::*;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Values {
        optional_number: Option<u32>,
        optional_string: Option<String>,
        required_string: String,
    }

    #[test]
    fn empty_values_are_none_only_for_options() {
        let values = from_bytes::<Values>(b"optional_number=&optional_string=&required_string=")
            .expect("valid values");

        assert_eq!(
            values,
            Values {
                optional_number: None,
                optional_string: None,
                required_string: String::new(),
            }
        );
    }

    #[test]
    fn non_empty_values_keep_existing_behavior() {
        let values =
            from_bytes::<Values>(b"optional_number=42&optional_string=hello&required_string=world")
                .expect("valid values");

        assert_eq!(
            values,
            Values {
                optional_number: Some(42),
                optional_string: Some("hello".to_owned()),
                required_string: "world".to_owned(),
            }
        );
    }

    #[test]
    fn missing_values_keep_existing_behavior() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct OptionalValue {
            value: Option<u32>,
        }

        assert_eq!(
            from_bytes::<OptionalValue>(b"").expect("valid values"),
            OptionalValue { value: None }
        );
    }

    #[test]
    fn invalid_non_empty_values_are_rejected() {
        let error = from_bytes::<Values>(
            b"optional_number=nope&optional_string=hello&required_string=world",
        )
        .expect_err("an invalid number is rejected");

        assert_eq!(error.path().to_string(), "optional_number");
    }

    #[test]
    fn sequences_of_pairs_keep_existing_behavior() {
        let pairs = from_bytes::<Vec<(String, u32)>>(b"a=1&b=2").expect("valid values");

        assert_eq!(pairs, vec![("a".to_owned(), 1), ("b".to_owned(), 2)]);
    }

    #[test]
    fn empty_map_keys_keep_existing_behavior() {
        let values =
            from_bytes::<BTreeMap<Option<String>, String>>(b"=value").expect("valid values");

        assert_eq!(
            values.get(&Some(String::new())).map(String::as_str),
            Some("value")
        );
        assert!(!values.contains_key(&None));
    }

    #[test]
    fn scalar_types_match_serde_urlencoded() {
        #[derive(Debug, Deserialize, PartialEq)]
        enum Choice {
            First,
            Second,
        }

        #[derive(Debug, Deserialize, PartialEq)]
        struct Scalars {
            boolean: bool,
            unsigned: u64,
            signed: i64,
            float: f64,
            character: char,
            string: String,
            choice: Choice,
        }

        let bytes = b"boolean=true&unsigned=42&signed=-7&float=1.5&character=x&string=hello+world&choice=Second";
        let expected = serde_urlencoded::from_bytes::<Scalars>(bytes).expect("valid values");

        assert_eq!(
            from_bytes::<Scalars>(bytes).expect("valid values"),
            expected
        );
    }
}
