use std::ops::Deref;

use ref_cast::RefCast;
use serde::{Deserialize, Serialize, de};

use crate::{
    Signal, SignalId, StrSurrogate, Surrogate, Surrogated, impl_surrogate, impl_surrogate_mut,
    impl_surrogate_ref,
};

#[derive(Debug, RefCast)]
#[repr(transparent)]
pub struct SignalSurrogate<T>(Signal<T>);

impl<T> SignalSurrogate<T> {
    #[inline]
    pub(crate) const fn new(v: Signal<T>) -> Self {
        Self(v)
    }
}

impl<T> SignalSurrogate<T>
where
    for<'b> &'b T: Surrogated,
{
    /// Borrows the current value.
    ///
    /// Reads inside a runtime expression are the client-reactive path and
    /// do not register a dependency on the server; the server-side
    /// evaluation that produces the initial render reads untracked.
    #[must_use]
    pub fn read(&self) -> <&T as Surrogated>::Surrogate {
        self.0.read_untracked().into_surrogate()
    }
}

impl<T> SignalSurrogate<T>
where
    T: Surrogated + Clone,
{
    /// Clones the current value.
    ///
    /// Like [`read`](Self::read), this does not register a dependency on
    /// the server.
    #[must_use]
    pub fn get(&self) -> <T as Surrogated>::Surrogate {
        self.0.get_untracked().into_surrogate()
    }
}

impl<T> SignalSurrogate<T>
where
    T: Surrogated,
{
    /// Writes a new value to the signal.
    ///
    /// # Panics
    ///
    /// Always panics; signal writes can only occur in client-side expressions.
    #[track_caller]
    pub fn set(&self, _v: T::Surrogate) {
        write_in_browser_only();
    }
}

impl SignalSurrogate<bool> {
    /// Replaces the value with its negation.
    ///
    /// # Panics
    ///
    /// Always panics; signal writes can only occur in client-side expressions.
    #[track_caller]
    pub fn toggle(&self) {
        write_in_browser_only();
    }
}

impl SignalSurrogate<f64> {
    /// Adds one to the value.
    ///
    /// # Panics
    ///
    /// Always panics; signal writes can only occur in client-side expressions.
    #[track_caller]
    pub fn increment(&self) {
        write_in_browser_only();
    }

    /// Subtracts one from the value.
    ///
    /// # Panics
    ///
    /// Always panics; signal writes can only occur in client-side expressions.
    #[track_caller]
    pub fn decrement(&self) {
        write_in_browser_only();
    }
}

impl SignalSurrogate<String> {
    /// Appends a string to the end of the value.
    ///
    /// The argument is anything that dereferences to a string, so both a
    /// borrowed `&str` and an owned `String` work. The owned form is what an
    /// event field yields: `Event::target.value` is a `String`, so
    /// `message.push_str(e.target.value)` is the common call.
    ///
    /// # Panics
    ///
    /// Always panics; signal writes can only occur in client-side expressions.
    #[track_caller]
    pub fn push_str(&self, _s: impl Deref<Target = StrSurrogate>) {
        write_in_browser_only();
    }
}

/// The panic shared by every signal write evaluated on the server.
#[track_caller]
fn write_in_browser_only() -> ! {
    panic!("expressions in which a signal is written to cannot be run server-side");
}

impl_surrogate!({T} Signal<T>, SignalSurrogate<T>);
impl_surrogate_ref!({T} Signal<T>, SignalSurrogate<T>);
impl_surrogate_mut!({T} Signal<T>, SignalSurrogate<T>);

impl<T> Serialize for SignalSurrogate<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        struct TaggedSignal {
            t: &'static str,
            id: std::string::String,
        }

        TaggedSignal {
            t: "Signal",
            id: self.0.id().to_string(),
        }
        .serialize(serializer)
    }
}

/// A signal sent by the client, as an argument to a run: its id next to its
/// current value.
///
/// The value is required. A run cannot read a signal it has no value for,
/// so a client that sends only the id is rejected the same way as one that
/// sends a value of the wrong shape.
impl<'de, T> Deserialize<'de> for SignalSurrogate<T>
where
    T: Surrogated,
    T::Surrogate: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields, bound(deserialize = "V: Deserialize<'de>"))]
        struct TaggedSignal<V> {
            t: std::string::String,
            id: SignalId,
            v: V,
        }

        let tagged = TaggedSignal::<T::Surrogate>::deserialize(deserializer)?;
        if tagged.t != "Signal" {
            return Err(de::Error::invalid_value(
                de::Unexpected::Str(&tagged.t),
                &"Signal",
            ));
        }
        Ok(Self(Signal::new(tagged.id, tagged.v.into_real())))
    }
}

#[cfg(test)]
mod tests {
    use std::panic::Location;

    use serde_json::json;

    use super::*;

    /// Builds a signal surrogate around a fresh signal holding `value`.
    #[track_caller]
    fn surrogate<T>(value: T) -> SignalSurrogate<T> {
        SignalSurrogate::new(Signal::new(SignalId::derive(Location::caller()), value))
    }

    #[test]
    fn serializes_as_its_id() {
        let signal = surrogate(String::from("shoes"));

        assert_eq!(
            serde_json::to_value(&signal).unwrap(),
            json!({ "t": "Signal", "id": signal.0.id().to_string() })
        );
    }

    #[test]
    fn deserializes_from_its_id_and_value() {
        let id = SignalId::derive(Location::caller());

        let signal: SignalSurrogate<String> =
            serde_json::from_value(json!({ "t": "Signal", "id": id.to_string(), "v": "shoes" }))
                .unwrap();

        assert_eq!(signal.0.id(), id);
        assert_eq!(*signal.0.read_untracked(), "shoes");
    }

    #[test]
    fn deserializes_a_value_through_its_surrogate() {
        let id = SignalId::derive(Location::caller());

        let signal: SignalSurrogate<Option<f64>> = serde_json::from_value(json!({
            "t": "Signal",
            "id": id.to_string(),
            "v": { "t": "Option", "v": 5.0 },
        }))
        .unwrap();

        assert_eq!(*signal.0.read_untracked(), Some(5.0));
    }

    #[test]
    fn rejects_another_tag() {
        let id = SignalId::derive(Location::caller());

        let error = serde_json::from_value::<SignalSurrogate<String>>(
            json!({ "t": "Procedure", "id": id.to_string(), "v": "shoes" }),
        )
        .unwrap_err();

        assert!(error.to_string().contains("expected Signal"), "{error}");
    }

    #[test]
    fn rejects_a_missing_value() {
        let id = SignalId::derive(Location::caller());

        let error = serde_json::from_value::<SignalSurrogate<String>>(
            json!({ "t": "Signal", "id": id.to_string() }),
        )
        .unwrap_err();

        assert!(error.to_string().contains("missing field `v`"), "{error}");
    }

    #[test]
    fn rejects_a_value_of_another_type() {
        let id = SignalId::derive(Location::caller());

        serde_json::from_value::<SignalSurrogate<f64>>(json!({
            "t": "Signal",
            "id": id.to_string(),
            "v": "shoes",
        }))
        .unwrap_err();
    }

    #[test]
    fn rejects_a_malformed_id() {
        serde_json::from_value::<SignalSurrogate<String>>(json!({
            "t": "Signal",
            "id": "not hex",
            "v": "shoes",
        }))
        .unwrap_err();
    }

    #[test]
    #[should_panic(expected = "cannot be run server-side")]
    fn toggle_panics_server_side() {
        surrogate(false).toggle();
    }

    #[test]
    #[should_panic(expected = "cannot be run server-side")]
    fn increment_panics_server_side() {
        surrogate(0.0).increment();
    }

    #[test]
    #[should_panic(expected = "cannot be run server-side")]
    fn decrement_panics_server_side() {
        surrogate(0.0).decrement();
    }

    #[test]
    #[should_panic(expected = "cannot be run server-side")]
    fn push_str_panics_server_side() {
        surrogate(String::new()).push_str(StrSurrogate::ref_cast(""));
    }
}
