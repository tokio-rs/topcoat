use std::string::String as StdString;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::JsCallable;

/// A string carried through `expr!` reactive expressions. The inherent method
/// surface is intentionally narrow — only methods that also have a JS
/// equivalent live here, so calls Rust can't lower to JS fail at compile time
/// instead of panicking in the browser.
///
/// Wire format is identical to `std::string::String` (transparent serde), so
/// signals shipping a `String` arrive in the browser as a plain JS string and
/// the JS-side `.clone()` is a no-op (strings are value-typed in JS).
#[derive(Debug, Clone)]
pub struct String {
    inner: StdString,
}

impl String {
    #[inline]
    pub fn new(inner: StdString) -> Self {
        Self { inner }
    }
}

impl From<&str> for String {
    #[inline]
    fn from(value: &str) -> Self {
        Self::new(value.to_owned())
    }
}

impl From<StdString> for String {
    #[inline]
    fn from(value: StdString) -> Self {
        Self::new(value)
    }
}

impl From<String> for StdString {
    #[inline]
    fn from(value: String) -> Self {
        value.inner
    }
}

impl From<&String> for StdString {
    #[inline]
    fn from(value: &String) -> Self {
        value.inner.clone()
    }
}

impl std::fmt::Display for String {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl Serialize for String {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        self.inner.serialize(ser)
    }
}

impl<'de> Deserialize<'de> for String {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        StdString::deserialize(de).map(Self::new)
    }
}

impl JsCallable for String {
    fn js_call(method: &str, _out: &mut StdString) {
        match method {
            // Strings are value-typed in JS; `.clone()` is the identity, so
            // we append nothing to the already-emitted receiver.
            "clone" => {}
            other => unreachable!(
                "method `{other}` reached JS codegen but is not implemented for `runtime::String`"
            ),
        }
    }
}
