use ref_cast::RefCast;
use serde::Serialize;

use crate::{
    Signal, StrSurrogate, Surrogated, impl_surrogate, impl_surrogate_mut, impl_surrogate_ref,
};

#[derive(RefCast)]
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
    pub fn read(&self) -> <&T as Surrogated>::Surrogate {
        self.0.read().into_surrogate()
    }
}

impl<T> SignalSurrogate<T>
where
    T: Surrogated + Clone,
{
    pub fn get(&self) -> <T as Surrogated>::Surrogate {
        self.0.get().into_surrogate()
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
    pub fn increment(&self) {
        write_in_browser_only();
    }

    /// Subtracts one from the value.
    ///
    /// # Panics
    ///
    /// Always panics; signal writes can only occur in client-side expressions.
    pub fn decrement(&self) {
        write_in_browser_only();
    }
}

impl SignalSurrogate<String> {
    /// Appends a string to the end of the value.
    ///
    /// # Panics
    ///
    /// Always panics; signal writes can only occur in client-side expressions.
    pub fn push_str(&self, _s: &StrSurrogate) {
        write_in_browser_only();
    }
}

/// The panic shared by every signal write evaluated on the server.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "cannot be run server-side")]
    fn toggle_panics_server_side() {
        SignalSurrogate::new(Signal::new(false)).toggle();
    }

    #[test]
    #[should_panic(expected = "cannot be run server-side")]
    fn increment_panics_server_side() {
        SignalSurrogate::new(Signal::new(0.0)).increment();
    }

    #[test]
    #[should_panic(expected = "cannot be run server-side")]
    fn decrement_panics_server_side() {
        SignalSurrogate::new(Signal::new(0.0)).decrement();
    }

    #[test]
    #[should_panic(expected = "cannot be run server-side")]
    fn push_str_panics_server_side() {
        SignalSurrogate::new(Signal::new(String::new())).push_str(StrSurrogate::ref_cast(""));
    }
}
