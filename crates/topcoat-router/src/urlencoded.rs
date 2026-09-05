//! A `serde` deserializer for `application/x-www-form-urlencoded` data.
//!
//! It mirrors `serde_urlencoded` with one difference: an empty value, as a
//! browser sends for a blank input, deserializes to `None` for an `Option<T>`
//! field. `serde_urlencoded` hands the empty string to `T` instead, so only
//! `Option<String>` accepts a blank input.

use std::borrow::Cow;
use std::fmt::Display;
use std::str::FromStr;

use serde::de::value::MapDeserializer;
use serde::de::{self, IntoDeserializer};
use serde::forward_to_deserialize_any;

/// The error type, the same one `serde_urlencoded` reports.
pub(crate) type Error = serde::de::value::Error;

/// Deserializes the pairs of a form body or a query string into a `T`.
pub(crate) struct Deserializer<'de> {
    inner: MapDeserializer<'de, Pairs<'de>, Error>,
}

impl<'de> Deserializer<'de> {
    /// Wraps the parsed pairs of a form body or a query string.
    pub(crate) fn new(pairs: form_urlencoded::Parse<'de>) -> Self {
        Self {
            inner: MapDeserializer::new(Pairs(pairs)),
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
        self.inner.end().and_then(|()| visitor.visit_unit())
    }

    forward_to_deserialize_any! {
        bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char str string option bytes
        byte_buf unit_struct newtype_struct tuple_struct struct identifier tuple
        enum ignored_any
    }
}

/// The parsed pairs, each value wrapped in a [`Value`].
struct Pairs<'de>(form_urlencoded::Parse<'de>);

impl<'de> Iterator for Pairs<'de> {
    type Item = (Cow<'de, str>, Value<'de>);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(key, value)| (key, Value(value)))
    }
}

/// One value of a form body or a query string.
struct Value<'de>(Cow<'de, str>);

impl<'de> IntoDeserializer<'de, Error> for Value<'de> {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

/// Parses `value` as a `T` and hands the result to `visit`.
fn parsed<'de, T, V>(
    value: &str,
    visitor: V,
    visit: fn(V, T) -> Result<V::Value, Error>,
) -> Result<V::Value, Error>
where
    T: FromStr,
    T::Err: Display,
    V: de::Visitor<'de>,
{
    value
        .parse::<T>()
        .map_err(de::Error::custom)
        .and_then(|parsed| visit(visitor, parsed))
}

impl<'de> de::Deserializer<'de> for Value<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.0 {
            Cow::Borrowed(value) => visitor.visit_borrowed_str(value),
            Cow::Owned(value) => visitor.visit_string(value),
        }
    }

    /// An empty value is `None`. Any other value is `Some` of the parsed `T`.
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        if self.0.is_empty() {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.0
            .into_deserializer()
            .deserialize_enum(name, variants, visitor)
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

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        parsed(&self.0, visitor, V::visit_bool::<Error>)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        parsed(&self.0, visitor, V::visit_u8::<Error>)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        parsed(&self.0, visitor, V::visit_u16::<Error>)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        parsed(&self.0, visitor, V::visit_u32::<Error>)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        parsed(&self.0, visitor, V::visit_u64::<Error>)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        parsed(&self.0, visitor, V::visit_i8::<Error>)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        parsed(&self.0, visitor, V::visit_i16::<Error>)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        parsed(&self.0, visitor, V::visit_i32::<Error>)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        parsed(&self.0, visitor, V::visit_i64::<Error>)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        parsed(&self.0, visitor, V::visit_f32::<Error>)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        parsed(&self.0, visitor, V::visit_f64::<Error>)
    }

    forward_to_deserialize_any! {
        char str string unit bytes byte_buf unit_struct tuple_struct struct
        identifier tuple ignored_any seq map
    }
}
