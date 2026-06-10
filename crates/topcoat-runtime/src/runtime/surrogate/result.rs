use ref_cast::RefCast;
use serde::{Deserialize, Serialize};

use crate::runtime::{
    Bool, Option, Str, Surrogate, Surrogated, impl_surrogate, impl_surrogate_mut,
    impl_surrogate_ref,
};

#[derive(Debug, Clone, RefCast)]
#[repr(transparent)]
pub struct Result<T, E>(std::result::Result<T, E>);

impl<T, E> Result<T, E> {
    #[inline]
    pub(crate) const fn new(v: std::result::Result<T, E>) -> Self {
        Self(v)
    }

    #[inline]
    pub fn is_ok(&self) -> Bool {
        Bool::new(self.0.is_ok())
    }

    #[inline]
    pub fn is_err(&self) -> Bool {
        Bool::new(self.0.is_err())
    }

    #[inline]
    pub fn ok(self) -> Option<T> {
        Option::new(self.0.ok())
    }

    #[inline]
    pub fn err(self) -> Option<E> {
        Option::new(self.0.err())
    }
}

impl<T, E> Result<T, E> {
    #[inline]
    pub fn from_ok(v: impl Surrogate<Real = T>) -> Self {
        Self(core::result::Result::Ok(v.into_real()))
    }
}

impl<T, E> Result<T, E>
where
    T: Surrogated,
    E: std::fmt::Debug,
{
    #[inline]
    pub fn unwrap(self) -> T::Surrogate {
        self.0.unwrap().into_surrogate()
    }

    #[inline]
    pub fn expect(self, msg: &Str) -> T::Surrogate {
        self.0.expect(&msg.0).into_surrogate()
    }
}

impl<T, E> Result<T, E> {
    #[inline]
    pub fn from_err(v: impl Surrogate<Real = E>) -> Self {
        Self(std::result::Result::Err(v.into_real()))
    }
}

impl<T, E> Result<T, E>
where
    T: std::fmt::Debug,
    E: Surrogated,
{
    #[inline]
    pub fn unwrap_err(self) -> E::Surrogate {
        self.0.unwrap_err().into_surrogate()
    }

    #[inline]
    pub fn expect_err(self, msg: &Str) -> E::Surrogate {
        self.0.expect_err(&msg.to_string()).into_surrogate()
    }
}

impl_surrogate!({T, E} std::result::Result<T, E>, Result<T, E>);
impl_surrogate_ref!({T, E} std::result::Result<T, E>, Result<T, E>);
impl_surrogate_mut!({T, E} std::result::Result<T, E>, Result<T, E>);

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum Body<T, E> {
    Ok { t: String, ok: T },
    Err { t: String, err: E },
}

impl<T, E> serde::Serialize for Result<T, E>
where
    for<'a> &'a T: Surrogated,
    for<'a> <&'a T as Surrogated>::Surrogate: serde::Serialize,
    for<'a> &'a E: Surrogated,
    for<'a> <&'a E as Surrogated>::Surrogate: serde::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let body = match &self.0 {
            std::result::Result::Ok(v) => Body::Ok {
                t: "Result".to_owned(),
                ok: v.into_surrogate(),
            },
            std::result::Result::Err(e) => Body::Err {
                t: "Result".to_owned(),
                err: e.into_surrogate(),
            },
        };
        body.serialize(serializer)
    }
}

impl<'de, T, E> serde::Deserialize<'de> for Result<T, E>
where
    T: Surrogated,
    T::Surrogate: serde::Deserialize<'de>,
    E: Surrogated,
    E::Surrogate: serde::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let body = Body::<T::Surrogate, E::Surrogate>::deserialize(deserializer)?;
        Ok(Self(match body {
            Body::Ok { ok, .. } => core::result::Result::Ok(ok.into_real()),
            Body::Err { err, .. } => core::result::Result::Err(err.into_real()),
        }))
    }
}
