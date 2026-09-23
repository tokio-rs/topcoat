use std::ops::Deref;

use ref_cast::RefCast;
use serde::{Deserialize, Serialize, de};

use crate::{
    Signal, SignalId, StrSurrogate, Surrogate, Surrogated, impl_surrogate, impl_surrogate_mut,
    impl_surrogate_ref,
};

/// A [`Signal`] in a runtime expression.
///
/// Reads work on both sides. Writes only work in the browser, so they
/// belong in event handler closures, whose bodies never run on the server.
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
    /// The expression reading it runs again in the browser when the signal
    /// changes. Unlike [`Signal::read`], this does not make the enclosing
    /// page or shard re-run on the server.
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
    /// Like [`read`](Self::read), this does not make the enclosing page or
    /// shard re-run on the server.
    #[must_use]
    pub fn get(&self) -> <T as Surrogated>::Surrogate {
        self.0.get_untracked().into_surrogate()
    }
}

impl<T> SignalSurrogate<T>
where
    T: Surrogated,
{
    /// Replaces the signal's value.
    ///
    /// # Panics
    ///
    /// Always panics on the server, since signals can only be written in the
    /// browser.
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
    /// Always panics on the server, since signals can only be written in the
    /// browser.
    #[track_caller]
    pub fn toggle(&self) {
        write_in_browser_only();
    }
}

macro_rules! numeric_signal {
    ($($number:ty),+ $(,)?) => {
        $(impl SignalSurrogate<$number> {
            /// Adds one to the value.
            ///
            /// # Panics
            ///
            /// Always panics on the server, since signals can only be written
            /// in the browser. Integer overflow panics in the browser.
            #[track_caller]
            pub fn increment(&self) {
                write_in_browser_only();
            }

            /// Subtracts one from the value.
            ///
            /// # Panics
            ///
            /// Always panics on the server, since signals can only be written
            /// in the browser. Integer overflow panics in the browser.
            #[track_caller]
            pub fn decrement(&self) {
                write_in_browser_only();
            }
        })+
    };
}

numeric_signal!(
    f64, u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize
);

impl SignalSurrogate<String> {
    /// Appends a string to the end of the value.
    ///
    /// Both a borrowed `&str` and an owned `String` work, so an event field
    /// like `e.target.value` can be passed directly:
    /// `message.push_str(e.target.value)`.
    ///
    /// # Panics
    ///
    /// Always panics on the server, since signals can only be written in the
    /// browser.
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

/// A signal the browser sends as an argument to a re-run: its id together
/// with its current value.
///
/// The value is required. A re-run cannot read a signal it has no value for,
/// so a request that sends only the id is rejected, like one that sends a
/// value of the wrong type.
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
    use topcoat_core::identity::Identity;

    use super::*;

    /// Builds a signal surrogate around a fresh signal holding `value`.
    #[track_caller]
    fn surrogate<T>(value: T) -> SignalSurrogate<T> {
        SignalSurrogate::new(Signal::new(
            SignalId::derive(Identity::ROOT, Location::caller()),
            value,
        ))
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
        let id = SignalId::derive(Identity::ROOT, Location::caller());

        let signal: SignalSurrogate<String> =
            serde_json::from_value(json!({ "t": "Signal", "id": id.to_string(), "v": "shoes" }))
                .unwrap();

        assert_eq!(signal.0.id(), id);
        assert_eq!(*signal.0.read_untracked(), "shoes");
    }

    #[test]
    fn deserializes_a_value_through_its_surrogate() {
        let id = SignalId::derive(Identity::ROOT, Location::caller());

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
        let id = SignalId::derive(Identity::ROOT, Location::caller());

        let error = serde_json::from_value::<SignalSurrogate<String>>(
            json!({ "t": "Procedure", "id": id.to_string(), "v": "shoes" }),
        )
        .unwrap_err();

        assert!(error.to_string().contains("expected Signal"), "{error}");
    }

    #[test]
    fn rejects_a_missing_value() {
        let id = SignalId::derive(Identity::ROOT, Location::caller());

        let error = serde_json::from_value::<SignalSurrogate<String>>(
            json!({ "t": "Signal", "id": id.to_string() }),
        )
        .unwrap_err();

        assert!(error.to_string().contains("missing field `v`"), "{error}");
    }

    #[test]
    fn rejects_a_value_of_another_type() {
        let id = SignalId::derive(Identity::ROOT, Location::caller());

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
