use std::ops::Deref;

use ref_cast::RefCast;
use serde::{Deserialize, de};

use super::sequence::deserialize_sequence;
use crate::{SliceSurrogate, Surrogated, impl_surrogate, impl_surrogate_mut, impl_surrogate_ref};

/// A fixed-size sequence of runtime values.
#[derive(Debug, Clone, Copy, RefCast)]
#[repr(transparent)]
pub struct ArraySurrogate<T, const N: usize>([T; N]);

impl<T, const N: usize> ArraySurrogate<T, N> {
    pub(crate) const fn new(value: [T; N]) -> Self {
        Self(value)
    }

    /// Borrows the array as a slice.
    #[must_use]
    pub fn as_slice(&self) -> &SliceSurrogate<T> {
        SliceSurrogate::ref_cast(self.0.as_slice())
    }
}

impl<T: Clone, const N: usize> ArraySurrogate<T, N> {
    /// Returns a copy of the array.
    #[must_use]
    pub fn to_owned(&self) -> Self {
        self.clone()
    }
}

impl<T, const N: usize> Deref for ArraySurrogate<T, N> {
    type Target = SliceSurrogate<T>;

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl_surrogate!({T, const N: usize} [T; N], ArraySurrogate<T, N>);
impl_surrogate_ref!({T, const N: usize} [T; N], ArraySurrogate<T, N>);
impl_surrogate_mut!({T, const N: usize} [T; N], ArraySurrogate<T, N>);

impl<T, const N: usize> serde::Serialize for ArraySurrogate<T, N>
where
    for<'a> &'a T: Surrogated,
    for<'a> <&'a T as Surrogated>::Surrogate: serde::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.as_slice().serialize_with_tag(serializer, "Array")
    }
}

impl<'de, T, const N: usize> Deserialize<'de> for ArraySurrogate<T, N>
where
    T: Surrogated,
    T::Surrogate: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let elements: Vec<T> = deserialize_sequence(deserializer, "Array")?;
        let values = elements.try_into().map_err(|values: Vec<T>| {
            de::Error::custom(format_args!(
                "expected {N} array elements, got {}",
                values.len()
            ))
        })?;
        Ok(Self(values))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::Surrogate;

    #[test]
    fn round_trips_arrays_of_any_length() {
        fn check<const N: usize>(values: [u128; N]) {
            let surrogate = values.into_surrogate();
            let wire = serde_json::to_value(surrogate).unwrap();
            assert_eq!(wire["t"], "Array");
            assert_eq!(wire["bits"], usize::BITS);
            let decoded: ArraySurrogate<u128, N> = serde_json::from_value(wire).unwrap();
            assert_eq!(decoded.into_real(), values);
            assert_eq!(surrogate.to_owned().into_real(), values);
            assert_eq!(surrogate.to_vec().into_real(), values);
        }
        check([]);
        check([u128::MAX]);
        check([42; 64]);
    }

    #[test]
    fn rejects_wrong_lengths_and_collection_kinds() {
        for wire in [
            json!({ "t": "Array", "bits": usize::BITS, "v": [] }),
            json!({ "t": "Array", "bits": usize::BITS, "v": [true, false] }),
            json!({ "t": "Vec", "bits": usize::BITS, "v": [true] }),
            json!({ "t": "Slice", "bits": usize::BITS, "v": [true] }),
        ] {
            assert!(serde_json::from_value::<ArraySurrogate<bool, 1>>(wire).is_err());
        }
    }
}
