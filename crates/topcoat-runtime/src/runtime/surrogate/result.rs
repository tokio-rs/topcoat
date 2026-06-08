use ref_cast::RefCast;

use crate::runtime::{
    Bool, Str, Surrogate, Surrogated, deserialize_tagged, impl_surrogate, impl_surrogate_mut,
    impl_surrogate_ref, serialize_tagged,
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
}

impl<T, E> Result<T, E>
where
    T: Surrogated,
    E: std::fmt::Debug,
{
    #[inline]
    pub fn ok(v: T::Surrogate) -> Self {
        Self(std::result::Result::Ok(v.into_real()))
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

impl<T, E> Result<T, E>
where
    T: std::fmt::Debug,
    E: Surrogated,
{
    #[inline]
    pub fn err(v: E::Surrogate) -> Self {
        Self(std::result::Result::Err(v.into_real()))
    }

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

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum Body<T, E> {
    Ok(T),
    Err(E),
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
            std::result::Result::Ok(v) => Body::Ok(v.into_surrogate()),
            std::result::Result::Err(e) => Body::Err(e.into_surrogate()),
        };
        serialize_tagged(serializer, "Result", &body)
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
        let body: Body<T::Surrogate, E::Surrogate> = deserialize_tagged(deserializer, "Result")?;
        Ok(Self(match body {
            Body::Ok(v) => core::result::Result::Ok(v.into_real()),
            Body::Err(e) => core::result::Result::Err(e.into_real()),
        }))
    }
}
