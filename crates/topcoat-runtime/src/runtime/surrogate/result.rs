use ref_cast::RefCast;
use serde::{Deserialize, Serialize};

use crate::runtime::{
    BoolSurrogate, OptionSurrogate, StrSurrogate, Surrogate, Surrogated, impl_surrogate,
    impl_surrogate_mut, impl_surrogate_ref,
};

#[derive(Debug, Clone, RefCast)]
#[repr(transparent)]
pub struct ResultSurrogate<T, E>(Result<T, E>);

impl<T, E> ResultSurrogate<T, E> {
    #[inline]
    pub(crate) const fn new(v: Result<T, E>) -> Self {
        Self(v)
    }

    #[inline]
    pub fn is_ok(&self) -> BoolSurrogate {
        BoolSurrogate::new(self.0.is_ok())
    }

    #[inline]
    pub fn is_err(&self) -> BoolSurrogate {
        BoolSurrogate::new(self.0.is_err())
    }

    #[inline]
    pub fn ok(self) -> OptionSurrogate<T> {
        OptionSurrogate::new(self.0.ok())
    }

    #[inline]
    pub fn err(self) -> OptionSurrogate<E> {
        OptionSurrogate::new(self.0.err())
    }
}

impl<T, E> ResultSurrogate<T, E> {
    #[inline]
    pub fn from_ok(v: impl Surrogate<Real = T>) -> Self {
        Self(Result::Ok(v.into_real()))
    }
}

impl<T, E> ResultSurrogate<T, E>
where
    T: Surrogated,
    E: std::fmt::Debug,
{
    /// Returns the contained `Ok` value.
    ///
    /// # Panics
    ///
    /// Panics if the result is `Err`.
    #[inline]
    pub fn unwrap(self) -> T::Surrogate {
        self.0.unwrap().into_surrogate()
    }

    /// Returns the contained `Ok` value, panicking with `msg` if `Err`.
    ///
    /// # Panics
    ///
    /// Panics with `msg` if the result is `Err`.
    #[inline]
    pub fn expect(self, msg: &StrSurrogate) -> T::Surrogate {
        self.0.expect(&msg.0).into_surrogate()
    }
}

impl<T, E> ResultSurrogate<T, E> {
    #[inline]
    pub fn from_err(v: impl Surrogate<Real = E>) -> Self {
        Self(Result::Err(v.into_real()))
    }
}

impl<T, E> ResultSurrogate<T, E>
where
    T: std::fmt::Debug,
    E: Surrogated,
{
    #[inline]
    pub fn unwrap_err(self) -> E::Surrogate {
        self.0.unwrap_err().into_surrogate()
    }

    #[inline]
    pub fn expect_err(self, msg: &StrSurrogate) -> E::Surrogate {
        self.0.expect_err(&msg.to_string()).into_surrogate()
    }
}

impl_surrogate!({T, E} Result<T, E>, ResultSurrogate<T, E>);
impl_surrogate_ref!({T, E} Result<T, E>, ResultSurrogate<T, E>);
impl_surrogate_mut!({T, E} Result<T, E>, ResultSurrogate<T, E>);

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum Body<T, E> {
    Ok { t: String, ok: T },
    Err { t: String, err: E },
}

impl<T, E> serde::Serialize for ResultSurrogate<T, E>
where
    for<'a> &'a T: Surrogated,
    for<'a> <&'a T as Surrogated>::Surrogate: serde::Serialize,
    for<'a> &'a E: Surrogated,
    for<'a> <&'a E as Surrogated>::Surrogate: serde::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let body = match &self.0 {
            Result::Ok(v) => Body::Ok {
                t: "Result".to_owned(),
                ok: v.into_surrogate(),
            },
            Result::Err(e) => Body::Err {
                t: "Result".to_owned(),
                err: e.into_surrogate(),
            },
        };
        body.serialize(serializer)
    }
}

impl<'de, T, E> serde::Deserialize<'de> for ResultSurrogate<T, E>
where
    T: Surrogated,
    T::Surrogate: serde::Deserialize<'de>,
    E: Surrogated,
    E::Surrogate: serde::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let body = Body::<T::Surrogate, E::Surrogate>::deserialize(deserializer)?;
        Ok(Self(match body {
            Body::Ok { ok, .. } => Result::Ok(ok.into_real()),
            Body::Err { err, .. } => Result::Err(err.into_real()),
        }))
    }
}
