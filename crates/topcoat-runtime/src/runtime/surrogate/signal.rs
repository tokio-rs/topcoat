use std::fmt::Write;

use ref_cast::RefCast;

use crate::runtime::{
    Signal, Surrogated, ToJs, impl_surrogate, impl_surrogate_mut, impl_surrogate_ref,
};

#[derive(RefCast)]
#[repr(transparent)]
pub struct WriteSignal<T>(Signal<T>);

impl<T> WriteSignal<T> {
    #[inline]
    pub(crate) const fn new(v: Signal<T>) -> Self {
        Self(v)
    }
}

impl<T> WriteSignal<T>
where
    T: Surrogated,
    for<'b> &'b T: Surrogated,
{
    pub fn read(&self) -> <&T as Surrogated>::Surrogate {
        self.0.read().into_surrogate()
    }

    pub fn set(&self, _v: T::Surrogate) {
        panic!("expressions in which a signal is written to cannot be run server-side");
    }
}

impl_surrogate!({T} Signal<T>, WriteSignal<T>);
impl_surrogate_ref!({T} Signal<T>, WriteSignal<T>);
impl_surrogate_mut!({T} Signal<T>, WriteSignal<T>);

impl<T> ToJs for &WriteSignal<T> {
    fn to_js(&self, out: &mut String) {
        let id = self.0.id();
        let _ = write!(out, "cx.signal(\"{id}\")");
    }
}
