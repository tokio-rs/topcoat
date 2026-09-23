mod _bool;
mod _f64;
mod _str;
mod array;
mod event;
mod integer;
mod option;
mod result;
mod sequence;
mod signal;
mod slice;
mod string;
mod tuple;
mod vec;

pub use _bool::*;
pub use _f64::*;
pub use _str::*;
pub use array::*;
pub use event::*;
pub use integer::*;
pub use option::*;
pub use result::*;
use serde::{Deserialize, Serialize, de};
pub use signal::*;
pub use slice::*;
pub use string::*;
pub use vec::*;

/// A type from the runtime vocabulary: a Rust type that runtime expressions
/// can use.
///
/// Inside a runtime expression, each value is replaced by its surrogate: a
/// wrapper that exposes only the operations the browser runtime can perform
/// the same way. Owned values, shared references, and mutable references
/// each have their own implementation.
pub trait Surrogated {
    /// The type that stands in for this type inside runtime expressions.
    type Surrogate: Surrogate<Real = Self>;

    /// Wraps the value in its surrogate.
    fn into_surrogate(self) -> Self::Surrogate;
}

/// The stand-in for a [`Surrogated`] type inside runtime expressions.
pub trait Surrogate {
    /// The type this surrogate stands in for.
    type Real: Surrogated<Surrogate = Self>;

    /// Unwraps the value this surrogate stands in for.
    fn into_real(self) -> Self::Real;
}

/// Implements [`Surrogated`] and [`Surrogate`] between an owned type and a
/// surrogate newtype around it.
///
/// The surrogate must be a tuple struct with an associated `new` function
/// that wraps the real value. Optional generic parameters go in braces
/// before the types, and an optional `where` clause follows them.
#[macro_export]
macro_rules! impl_surrogate {
    (
        $({$($g:tt)*})? $real:ty, $surrogate:ty
        $(where $($w:tt)*)?
    ) => {
        impl<$($($g)*)?> $crate::Surrogated for $real
        $(where $($w)*)?
        {
            type Surrogate = $surrogate;

            fn into_surrogate(self) -> Self::Surrogate {
                <$surrogate>::new(self)
            }
        }

        impl<$($($g)*)?> $crate::Surrogate for $surrogate
        $(where $($w)*)?
        {
            type Real = $real;

            fn into_real(self) -> Self::Real {
                self.0
            }
        }
    };
}

/// Implements [`Surrogated`] and [`Surrogate`] between a shared reference to
/// a type and a shared reference to its surrogate.
///
/// The surrogate must be a `#[repr(transparent)]` tuple struct deriving
/// `ref_cast::RefCast`. The syntax matches [`impl_surrogate!`].
#[macro_export]
macro_rules! impl_surrogate_ref {
    (
        $({$($g:tt)*})? $real:ty, $surrogate:ty
        $(where $($w:tt)*)?
    ) => {
        impl<'__lifetime, $($($g)*)?> $crate::Surrogated for &'__lifetime $real
        $(where $($w)*)?
        {
            type Surrogate = &'__lifetime $surrogate;

            fn into_surrogate(self) -> Self::Surrogate {
                <$surrogate>::ref_cast(self)
            }
        }

        impl<'__lifetime, $($($g)*)?> $crate::Surrogate for &'__lifetime $surrogate
        $(where $($w)*)?
        {
            type Real = &'__lifetime $real;

            fn into_real(self) -> Self::Real {
                &self.0
            }
        }
    };
}

/// Implements [`Surrogated`] and [`Surrogate`] between a mutable reference to
/// a type and a mutable reference to its surrogate.
///
/// The surrogate must be a `#[repr(transparent)]` tuple struct deriving
/// `ref_cast::RefCast`. The syntax matches [`impl_surrogate!`].
#[macro_export]
macro_rules! impl_surrogate_mut {
    (
        $({$($g:tt)*})? $real:ty, $surrogate:ty
        $(where $($w:tt)*)?
    ) => {
        impl<'__lifetime, $($($g)*)?> $crate::Surrogated for &'__lifetime mut $real
        $(where $($w)*)?
        {
            type Surrogate = &'__lifetime mut $surrogate;

            fn into_surrogate(self) -> Self::Surrogate {
                <$surrogate>::ref_cast_mut(self)
            }
        }

        impl<'__lifetime, $($($g)*)?> $crate::Surrogate for &'__lifetime mut $surrogate
        $(where $($w)*)?
        {
            type Real = &'__lifetime mut $real;

            fn into_real(self) -> Self::Real {
                &mut self.0
            }
        }
    };
}

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
) -> ::core::result::Result<S::Ok, S::Error>
where
    T: Serialize + ?Sized,
    S: serde::Serializer,
{
    TaggedRef { t: tag, v: value }.serialize(serializer)
}

pub(crate) fn deserialize_tagged<'de, T, D>(
    deserializer: D,
    expected: &'static str,
) -> ::core::result::Result<T, D::Error>
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
