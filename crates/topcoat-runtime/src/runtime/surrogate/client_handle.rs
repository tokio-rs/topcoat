use std::ops::Deref;

use ref_cast::RefCast;
use serde::Serialize;

use crate::runtime::{
    ClientHandle, ClientHandleId, Surrogated, impl_surrogate, impl_surrogate_mut,
    impl_surrogate_ref,
};

#[derive(RefCast)]
#[repr(transparent)]
pub struct ClientHandleSurrogate<T>(ClientHandle<T>);

impl<T> ClientHandleSurrogate<T> {
    #[inline]
    pub(crate) const fn new(v: ClientHandle<T>) -> Self {
        Self(v)
    }
}

impl_surrogate!({T} ClientHandle<T>, ClientHandleSurrogate<T>);
impl_surrogate_ref!({T} ClientHandle<T>, ClientHandleSurrogate<T>);
impl_surrogate_mut!({T} ClientHandle<T>, ClientHandleSurrogate<T>);

impl<T> Deref for ClientHandleSurrogate<T>
where
    T: Surrogated,
    for<'b> &'b T: Surrogated<Surrogate = &'b <T as Surrogated>::Surrogate>,
{
    type Target = <T as Surrogated>::Surrogate;

    fn deref(&self) -> &Self::Target {
        self.0.value().into_surrogate()
    }
}

impl<T> Serialize for ClientHandleSurrogate<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        struct TaggedClientHandle {
            t: &'static str,
            id: ClientHandleId,
        }

        TaggedClientHandle {
            t: "Handle",
            id: self.0.id(),
        }
        .serialize(serializer)
    }
}
