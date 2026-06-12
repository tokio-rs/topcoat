use ref_cast::RefCast;
use serde::{Serialize, ser::SerializeStruct};

use crate::runtime::{
    Signal, SignalId, Surrogated, impl_surrogate, impl_surrogate_mut, impl_surrogate_ref,
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
    pub fn set(&self, _v: T::Surrogate) {
        panic!("expressions in which a signal is written to cannot be run server-side");
    }
}

impl_surrogate!({T} Signal<T>, SignalSurrogate<T>);
impl_surrogate_ref!({T} Signal<T>, SignalSurrogate<T>);
impl_surrogate_mut!({T} Signal<T>, SignalSurrogate<T>);

impl<T> Serialize for SignalSurrogate<T>
where
    T: Surrogated,
    <T as Surrogated>::Surrogate: Serialize,
    for<'b> &'b T: Surrogated<Surrogate = &'b <T as Surrogated>::Surrogate>,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut fields = serializer.serialize_struct("Signal", 3)?;
        fields.serialize_field("t", "Signal")?;
        fields.serialize_field("id", &self.0.id())?;
        fields.serialize_field("v", self.read())?;
        fields.end()
    }
}
