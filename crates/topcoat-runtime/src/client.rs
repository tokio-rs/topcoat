//! Values supported by the experimental typed Wasm bridge.

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use topcoat_core::context::Cx;
use topcoat_view::{AttributeValueViewParts, NodeViewParts, PartsWriter};

/// Immutable Unicode text that stays in JavaScript during client evaluation.
///
/// The server stores native text. The Wasm backend uses a JavaScript string
/// reference, including for cloning, comparison, and concatenation. It does
/// not expose a borrowed UTF-8 string or byte offsets.
///
/// Construct text on the server and capture it or store it in a signal. Client
/// expressions currently support cloning, equality, [`concat`](Self::concat),
/// and [`is_empty`](Self::is_empty).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Text(String);

impl Text {
    /// Whether the text contains no characters.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns this text followed by `other`, leaving both inputs unchanged.
    #[must_use]
    pub fn concat(&self, other: &Self) -> Self {
        let mut value = self.0.clone();
        value.push_str(&other.0);
        Self(value)
    }
}

impl From<&str> for Text {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl From<String> for Text {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl NodeViewParts for Text {
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_string(self.0);
    }
}

impl AttributeValueViewParts for Text {
    fn attribute_present(&self) -> bool {
        true
    }

    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_string(self.0);
    }
}

/// A value supported by the typed Wasm signal bridge.
///
/// Implementations are limited to [`Text`], booleans, unit, `f64`, and
/// fixed-width integers of at most 32 bits. Serialization is used for server
/// state transfer; browser-to-Wasm calls use primitives and text references.
/// Application-defined implementations are not supported yet.
pub trait ClientValue: sealed::Sealed + Clone + Serialize + DeserializeOwned {}

mod sealed {
    pub trait Sealed {}
}

macro_rules! client_values {
    ($($ty:ty),* $(,)?) => { $(
        impl sealed::Sealed for $ty {}
        impl ClientValue for $ty {}
    )* };
}

client_values!(Text, bool, (), f64, i8, i16, i32, u8, u16, u32);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_has_value_semantics_and_preserves_unicode() {
        let text = Text::from("\u{1f680}e\u{301}");
        let copy = text.clone();
        let combined = text.concat(&Text::from("\u{2615}"));
        assert_eq!(text, copy);
        assert_eq!(combined, Text::from("\u{1f680}e\u{301}\u{2615}"));
        assert!(!text.is_empty());
        assert!(Text::from("").is_empty());
    }

    #[test]
    fn text_round_trips_through_server_serialization() {
        let text = Text::from("<&>\u{1f680}\0");
        let encoded = serde_json::to_string(&text).unwrap();
        assert_eq!(serde_json::from_str::<Text>(&encoded).unwrap(), text);
        assert!(serde_json::from_str::<Text>("42").is_err());
    }
}
