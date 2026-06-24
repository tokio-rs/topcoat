use ref_cast::RefCast;

use crate::runtime::{
    BoolSurrogate, StrSurrogate, Surrogate, Surrogated, deserialize_tagged, impl_surrogate,
    impl_surrogate_mut, impl_surrogate_ref, serialize_tagged,
};

#[derive(Debug, Clone, RefCast)]
#[repr(transparent)]
pub struct OptionSurrogate<T>(Option<T>);

impl<T> OptionSurrogate<T> {
    #[inline]
    pub(crate) const fn new(v: Option<T>) -> Self {
        Self(v)
    }

    #[inline]
    pub fn none() -> Self {
        Self(None)
    }

    #[inline]
    pub fn is_some(&self) -> BoolSurrogate {
        BoolSurrogate::new(self.0.is_some())
    }

    #[inline]
    pub fn is_none(&self) -> BoolSurrogate {
        BoolSurrogate::new(self.0.is_none())
    }
}

impl<T> OptionSurrogate<T>
where
    T: Surrogated,
{
    #[inline]
    pub fn some(v: impl Surrogate<Real = T>) -> Self {
        Self(Some(v.into_real()))
    }

    /// Returns the contained value.
    ///
    /// # Panics
    ///
    /// Panics if the option is `None`.
    #[inline]
    pub fn unwrap(self) -> T::Surrogate {
        self.0.unwrap().into_surrogate()
    }

    /// Returns the contained value, panicking with `msg` if `None`.
    ///
    /// # Panics
    ///
    /// Panics with `msg` if the option is `None`.
    #[inline]
    pub fn expect(self, msg: &StrSurrogate) -> T::Surrogate {
        self.0.expect(&msg.0).into_surrogate()
    }
}

impl_surrogate!({T} Option<T>, OptionSurrogate<T>);
impl_surrogate_ref!({T} Option<T>, OptionSurrogate<T>);
impl_surrogate_mut!({T} Option<T>, OptionSurrogate<T>);

impl<T> serde::Serialize for OptionSurrogate<T>
where
    for<'a> &'a T: Surrogated,
    for<'a> <&'a T as Surrogated>::Surrogate: serde::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let inner: Option<<&T as Surrogated>::Surrogate> =
            self.0.as_ref().map(Surrogated::into_surrogate);
        serialize_tagged(serializer, "Option", &inner)
    }
}

impl<'de, T> serde::Deserialize<'de> for OptionSurrogate<T>
where
    T: Surrogated,
    T::Surrogate: serde::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let inner: Option<T::Surrogate> = deserialize_tagged(deserializer, "Option")?;
        Ok(Self(inner.map(Surrogate::into_real)))
    }
}
