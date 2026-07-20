use std::{iter::empty, ops::Deref};

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use topcoat_core::context::Cx;
use topcoat_view::{NodeViewParts, PartsWriter};
use uuid::Uuid;

use crate::{Surrogate, Surrogated};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SignalId(Uuid);

impl SignalId {
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for SignalId {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SignalId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug)]
pub struct Signal<T> {
    id: SignalId,
    value: T,
}

impl<T> Signal<T> {
    #[inline]
    pub fn new(value: T) -> Self {
        Self {
            id: SignalId::new(),
            value,
        }
    }

    pub(crate) fn id(&self) -> SignalId {
        self.id
    }

    pub(crate) fn read(&self) -> &T {
        &self.value
    }
}

impl<T> Signal<T>
where
    T: Clone,
{
    pub(crate) fn get(&self) -> T {
        self.value.clone()
    }
}

pub struct SignalDeclaration<'a, T>(&'a Signal<T>);

impl<'a, T> SignalDeclaration<'a, T> {
    #[inline]
    pub fn new(signal: &'a Signal<T>) -> Self {
        Self(signal)
    }
}

impl<T> NodeViewParts for SignalDeclaration<'_, T>
where
    for<'a> &'a T: Surrogated,
    for<'a> <&'a T as Surrogated>::Surrogate: Serialize,
{
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        #[derive(Serialize)]
        struct SignalDeclarationPayload<'a, V>
        where
            V: ?Sized,
        {
            t: &'static str,
            id: std::string::String,
            v: &'a V,
        }

        let value = (&self.0.value).into_surrogate();
        let payload = SignalDeclarationPayload {
            t: "signal",
            id: self.0.id().to_string(),
            v: &value,
        };
        let json = serde_json::to_string(&payload)
            .expect("failed to serialize signal declaration payload");

        parts.push_str_unescaped("<!-- ::topcoat::signal(");
        parts.push_str_unescaped(escape_for_html_comment(&json));
        parts.push_str_unescaped(") -->");
    }
}

/// Rewrites the HTML-significant characters in a serialized JSON payload as
/// JSON `\uXXXX` escapes so the payload cannot terminate the HTML comment it is
/// embedded in.
///
/// The client reads this marker back with `JSON.parse` (see the browser
/// runtime's `comment.ts`), which decodes `\uXXXX` escapes, so the value is
/// byte-identical after the round trip. `serde_json` does not escape `<`, `>`,
/// or `&`, so without this a signal value containing `-->` would close the
/// comment early and drop the browser back into live HTML parsing -- an XSS
/// vector. `"` is already emitted as `\"` by `serde_json`, so structural quotes
/// are left untouched.
fn escape_for_html_comment(json: &str) -> String {
    json.replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026")
}

#[derive(Debug, Clone)]
pub struct ReadSignal<T> {
    id: SignalId,
    value: T,
}

impl<T> ReadSignal<T> {
    pub fn new(signal: &Signal<T>) -> Self
    where
        T: Clone,
    {
        Self {
            id: signal.id,
            value: signal.value.clone(),
        }
    }

    pub fn id(&self) -> SignalId {
        self.id
    }
}

impl<T> Deref for ReadSignal<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<'de, T> Deserialize<'de> for ReadSignal<T>
where
    T: Surrogated,
    T::Surrogate: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct EncodedReadSignal<S> {
            id: SignalId,
            value: S,
        }

        let encoded = EncodedReadSignal::<T::Surrogate>::deserialize(deserializer)?;
        Ok(Self {
            id: encoded.id,
            value: encoded.value.into_real(),
        })
    }
}

pub trait Signals: Sized {
    fn ids(&self) -> impl Iterator<Item = SignalId>;
    fn decode(encoded_signals: EncodedSignals) -> Self;
}

impl Signals for () {
    fn ids(&self) -> impl Iterator<Item = SignalId> {
        empty()
    }

    fn decode(_encoded_signals: EncodedSignals) -> Self {}
}

macro_rules! impl_signals_for_tuple {
    ($($n:tt $t:ident),+) => {
        impl<$($t),+> Signals for ($(ReadSignal<$t>,)+)
        where
            $(
                $t: Surrogated,
                <$t as Surrogated>::Surrogate: DeserializeOwned,
            )+
        {
            fn ids(&self) -> impl Iterator<Item = SignalId> {
                [$(self.$n.id),+].into_iter()
            }

            fn decode(encoded_signals: EncodedSignals) -> Self {
                serde_json::from_str(&encoded_signals.0).unwrap()
            }
        }
    };
}

impl_signals_for_tuple!(0 T0);
impl_signals_for_tuple!(0 T0, 1 T1);
impl_signals_for_tuple!(0 T0, 1 T1, 2 T2);
impl_signals_for_tuple!(0 T0, 1 T1, 2 T2, 3 T3);
impl_signals_for_tuple!(0 T0, 1 T1, 2 T2, 3 T3, 4 T4);
impl_signals_for_tuple!(0 T0, 1 T1, 2 T2, 3 T3, 4 T4, 5 T5);
impl_signals_for_tuple!(0 T0, 1 T1, 2 T2, 3 T3, 4 T4, 5 T5, 6 T6);
impl_signals_for_tuple!(0 T0, 1 T1, 2 T2, 3 T3, 4 T4, 5 T5, 6 T6, 7 T7);
impl_signals_for_tuple!(0 T0, 1 T1, 2 T2, 3 T3, 4 T4, 5 T5, 6 T6, 7 T7, 8 T8);
impl_signals_for_tuple!(0 T0, 1 T1, 2 T2, 3 T3, 4 T4, 5 T5, 6 T6, 7 T7, 8 T8, 9 T9);
impl_signals_for_tuple!(0 T0, 1 T1, 2 T2, 3 T3, 4 T4, 5 T5, 6 T6, 7 T7, 8 T8, 9 T9, 10 T10);
impl_signals_for_tuple!(0 T0, 1 T1, 2 T2, 3 T3, 4 T4, 5 T5, 6 T6, 7 T7, 8 T8, 9 T9, 10 T10, 11 T11);

pub struct EncodedSignals(String);

impl EncodedSignals {
    pub fn new(inner: impl Into<String>) -> Self {
        Self(inner.into())
    }
}

#[cfg(test)]
mod tests {
    use topcoat_view::{HtmlContext, NodeViewParts, PartsWriter, View, ViewParts};

    use super::*;

    fn render_signal(value: &str) -> String {
        let signal = Signal::new(value.to_string());
        let mut parts = ViewParts::new();
        SignalDeclaration::new(&signal).into_view_parts(
            &Cx::default(),
            &mut PartsWriter::new(&mut parts, HtmlContext::Text),
        );
        View::new(parts).render(&Cx::default())
    }

    #[test]
    fn signal_payload_cannot_terminate_the_comment() {
        let rendered = render_signal("--><img src=x onerror=alert(1)>");
        // The only "-->" is the marker's own closing delimiter.
        assert_eq!(rendered.matches("-->").count(), 1);
        assert!(rendered.trim_end().ends_with(") -->"));
        assert!(!rendered.contains("<img"));
    }

    #[test]
    fn signal_payload_round_trips_through_json() {
        let value = "--></script> a < b && \"quoted\"";
        let rendered = render_signal(value);
        let payload = rendered
            .split_once("::topcoat::signal(")
            .and_then(|(_, rest)| rest.rsplit_once(") -->"))
            .map(|(json, _)| json.trim())
            .expect("rendered marker has the expected shape");
        // The client parses this payload verbatim with JSON.parse, so it must
        // be valid JSON that decodes back to the original value unchanged.
        let parsed: serde_json::Value =
            serde_json::from_str(payload).expect("payload is valid JSON");
        assert_eq!(parsed["v"], value);
    }
}
