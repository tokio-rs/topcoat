use std::{
    fmt::{self, Display},
    marker::PhantomData,
    str::FromStr,
};

use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{self, Visitor},
};

use crate::{LanguageIdentifier, Locale};

/// Serializes a value as its [`Display`] text.
fn serialize_text<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    T: Display,
    S: Serializer,
{
    serializer.collect_str(value)
}

/// Deserializes a value by parsing a string with [`FromStr`].
fn deserialize_text<'de, T, D>(deserializer: D, expected: &'static str) -> Result<T, D::Error>
where
    T: FromStr<Err: Display>,
    D: Deserializer<'de>,
{
    deserializer.deserialize_str(TextVisitor {
        expected,
        _marker: PhantomData,
    })
}

struct TextVisitor<T> {
    expected: &'static str,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Visitor<'_> for TextVisitor<T>
where
    T: FromStr<Err: Display>,
{
    type Value = T;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.expected)
    }

    fn visit_str<E: de::Error>(self, text: &str) -> Result<T, E> {
        text.parse().map_err(E::custom)
    }
}

impl Serialize for Locale {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serialize_text(self, serializer)
    }
}

impl<'de> Deserialize<'de> for Locale {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserialize_text(deserializer, "a locale such as `en-US`")
    }
}

impl Serialize for LanguageIdentifier {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serialize_text(self, serializer)
    }
}

impl<'de> Deserialize<'de> for LanguageIdentifier {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserialize_text(deserializer, "a language identifier such as `en-US`")
    }
}

#[cfg(test)]
mod tests {
    use serde::de::{
        IntoDeserializer,
        value::{Error, StrDeserializer},
    };

    use super::*;
    use crate::{langid, locale};

    fn from_str<'de, T: Deserialize<'de>>(text: &'de str) -> Result<T, Error> {
        let deserializer: StrDeserializer<'_, Error> = text.into_deserializer();
        T::deserialize(deserializer)
    }

    #[test]
    fn deserializes_by_parsing() {
        assert_eq!(
            from_str::<Locale>("de-at-u-ca-buddhist").unwrap(),
            locale!("de-AT-u-ca-buddhist")
        );
        assert_eq!(
            from_str::<LanguageIdentifier>("zh-hant").unwrap(),
            langid!("zh-Hant")
        );
    }

    #[test]
    fn rejects_invalid_text() {
        assert!(from_str::<Locale>("en-u").is_err());
        assert!(from_str::<LanguageIdentifier>("en-u-ca-buddhist").is_err());
    }
}
