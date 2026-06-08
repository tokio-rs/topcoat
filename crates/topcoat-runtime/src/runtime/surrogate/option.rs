use ref_cast::RefCast;

use crate::runtime::{
    Bool, Str, Surrogate, Surrogated, deserialize_tagged, impl_surrogate, impl_surrogate_mut,
    impl_surrogate_ref, serialize_tagged,
};

#[derive(Debug, Clone, RefCast)]
#[repr(transparent)]
pub struct Option<T>(std::option::Option<T>);

impl<T> Option<T> {
    #[inline]
    pub(crate) const fn new(v: std::option::Option<T>) -> Self {
        Self(v)
    }

    #[inline]
    pub fn none() -> Self {
        Self(None)
    }

    #[inline]
    pub fn is_some(&self) -> Bool {
        Bool::new(self.0.is_some())
    }

    #[inline]
    pub fn is_none(&self) -> Bool {
        Bool::new(self.0.is_none())
    }
}

impl<T> Option<T>
where
    T: Surrogated,
{
    #[inline]
    pub fn some(v: impl Surrogate<Real = T>) -> Self {
        Self(Some(v.into_real()))
    }

    #[inline]
    pub fn unwrap(self) -> T::Surrogate {
        self.0.unwrap().into_surrogate()
    }

    #[inline]
    pub fn expect(self, msg: &Str) -> T::Surrogate {
        self.0.expect(&msg.0).into_surrogate()
    }
}

impl_surrogate!({T} std::option::Option<T>, Option<T>);
impl_surrogate_ref!({T} std::option::Option<T>, Option<T>);
impl_surrogate_mut!({T} std::option::Option<T>, Option<T>);

impl<T> serde::Serialize for Option<T>
where
    for<'a> &'a T: Surrogated,
    for<'a> <&'a T as Surrogated>::Surrogate: serde::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let inner: std::option::Option<<&T as Surrogated>::Surrogate> =
            self.0.as_ref().map(|v| v.into_surrogate());
        serialize_tagged(serializer, "Option", &inner)
    }
}

impl<'de, T> serde::Deserialize<'de> for Option<T>
where
    T: Surrogated,
    T::Surrogate: serde::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let inner: std::option::Option<T::Surrogate> = deserialize_tagged(deserializer, "Option")?;
        Ok(Self(inner.map(|s| s.into_real())))
    }
}
