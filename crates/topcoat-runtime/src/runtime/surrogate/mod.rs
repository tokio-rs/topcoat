mod _f64;
mod _i32;
mod _str;
mod event;
mod signal;
mod string;

use serde::{Deserialize, Serialize, de};

pub use _f64::*;
pub use _i32::*;
pub use _str::*;
pub use event::*;
pub use signal::*;
pub use string::*;

pub trait Surrogated {
    type Surrogate: Surrogate<Real = Self>;

    fn into_surrogate(self) -> Self::Surrogate;
}

pub trait Surrogate {
    type Real: Surrogated<Surrogate = Self>;

    fn into_real(self) -> Self::Real;
}

macro_rules! impl_surrogate {
    (
        $({$($g:tt)*})? $real:ty, $surrogate:ty
        $(where $($w:tt)*)?
    ) => {
        impl<$($($g)*)?> $crate::runtime::Surrogated for $real
        $(where $($w)*)?
        {
            type Surrogate = $surrogate;

            fn into_surrogate(self) -> Self::Surrogate {
                <$surrogate>::new(self)
            }
        }

        impl<$($($g)*)?> $crate::runtime::Surrogate for $surrogate
        $(where $($w)*)?
        {
            type Real = $real;

            fn into_real(self) -> Self::Real {
                self.0
            }
        }
    };
}
pub(crate) use impl_surrogate;

macro_rules! impl_surrogate_ref {
    (
        $({$($g:tt)*})? $real:ty, $surrogate:ty
        $(where $($w:tt)*)?
    ) => {
        impl<'__lifetime, $($($g)*)?> $crate::runtime::Surrogated for &'__lifetime $real
        $(where $($w)*)?
        {
            type Surrogate = &'__lifetime $surrogate;

            fn into_surrogate(self) -> Self::Surrogate {
                <$surrogate>::ref_cast(self)
            }
        }

        impl<'__lifetime, $($($g)*)?> $crate::runtime::Surrogate for &'__lifetime $surrogate
        $(where $($w)*)?
        {
            type Real = &'__lifetime $real;

            fn into_real(self) -> Self::Real {
                &self.0
            }
        }
    };
}
pub(crate) use impl_surrogate_ref;

macro_rules! impl_surrogate_mut {
    (
        $({$($g:tt)*})? $real:ty, $surrogate:ty
        $(where $($w:tt)*)?
    ) => {
        impl<'__lifetime, $($($g)*)?> $crate::runtime::Surrogated for &'__lifetime mut $real
        $(where $($w)*)?
        {
            type Surrogate = &'__lifetime mut $surrogate;

            fn into_surrogate(self) -> Self::Surrogate {
                <$surrogate>::ref_cast_mut(self)
            }
        }

        impl<'__lifetime, $($($g)*)?> $crate::runtime::Surrogate for &'__lifetime mut $surrogate
        $(where $($w)*)?
        {
            type Real = &'__lifetime mut $real;

            fn into_real(self) -> Self::Real {
                &mut self.0
            }
        }
    };
}
pub(crate) use impl_surrogate_mut;

#[derive(Serialize)]
struct TaggedRef<'a, T>
where
    T: ?Sized,
{
    t: &'static str,
    v: &'a T,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Tagged<T> {
    t: std::string::String,
    v: T,
}

pub(crate) fn serialize_tagged<T, S>(
    serializer: S,
    tag: &'static str,
    value: &T,
) -> Result<S::Ok, S::Error>
where
    T: Serialize + ?Sized,
    S: serde::Serializer,
{
    TaggedRef { t: tag, v: value }.serialize(serializer)
}

pub(crate) fn deserialize_tagged<'de, T, D>(
    deserializer: D,
    expected: &'static str,
) -> Result<T, D::Error>
where
    T: Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    let tagged = Tagged::<T>::deserialize(deserializer)?;
    if tagged.t == expected {
        Ok(tagged.v)
    } else {
        Err(de::Error::invalid_value(
            de::Unexpected::Str(&tagged.t),
            &expected,
        ))
    }
}
