use std::ops::Deref;

use ref_cast::RefCast;
use serde::Deserialize;

use super::sequence::deserialize_sequence;
use crate::{SliceSurrogate, Surrogated, impl_surrogate, impl_surrogate_mut, impl_surrogate_ref};

/// An owned sequence of runtime values.
#[derive(Debug, Clone, RefCast)]
#[repr(transparent)]
pub struct VecSurrogate<T>(Vec<T>);

impl<T> VecSurrogate<T> {
    pub(crate) const fn new(value: Vec<T>) -> Self {
        Self(value)
    }

    /// Borrows the vector as a slice.
    #[must_use]
    pub fn as_slice(&self) -> &SliceSurrogate<T> {
        SliceSurrogate::ref_cast(self.0.as_slice())
    }
}

impl<T> Deref for VecSurrogate<T> {
    type Target = SliceSurrogate<T>;

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl_surrogate!({T} Vec<T>, VecSurrogate<T>);
impl_surrogate_ref!({T} Vec<T>, VecSurrogate<T>);
impl_surrogate_mut!({T} Vec<T>, VecSurrogate<T>);

impl<T> serde::Serialize for VecSurrogate<T>
where
    for<'a> &'a T: Surrogated,
    for<'a> <&'a T as Surrogated>::Surrogate: serde::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.as_slice().serialize_with_tag(serializer, "Vec")
    }
}

impl<'de, T> Deserialize<'de> for VecSurrogate<T>
where
    T: Surrogated,
    T::Surrogate: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserialize_sequence(deserializer, "Vec").map(Self)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::Surrogate;

    #[test]
    fn serializes_owned_and_borrowed_sequences_through_element_surrogates() {
        let values = vec![0u128, u128::MAX];
        let elements = json!([
            { "t": "u128", "bits": 128, "v": "0" },
            { "t": "u128", "bits": 128, "v": u128::MAX.to_string() },
        ]);
        let owned = values.clone().into_surrogate();
        let wire = json!({ "t": "Vec", "bits": usize::BITS, "v": elements });
        assert_eq!(serde_json::to_value(&owned).unwrap(), wire);
        assert_eq!(
            serde_json::to_value((&values).into_surrogate()).unwrap(),
            wire
        );
        let decoded: VecSurrogate<u128> = serde_json::from_value(wire).unwrap();
        assert_eq!(decoded.into_real(), values);
        assert_eq!(
            serde_json::to_value(values.as_slice().into_surrogate()).unwrap(),
            json!({ "t": "Slice", "bits": usize::BITS, "v": elements }),
        );
    }

    #[test]
    fn round_trips_empty_and_nested_vectors() {
        for values in [
            vec![],
            vec![vec![], vec![Some(42u64), None, Some(u64::MAX)]],
        ] {
            let wire = serde_json::to_value(values.clone().into_surrogate()).unwrap();
            let decoded: VecSurrogate<Vec<Option<u64>>> = serde_json::from_value(wire).unwrap();
            assert_eq!(decoded.into_real(), values);
        }
    }

    #[test]
    fn rejects_invalid_payloads() {
        for wire in [
            json!([]),
            json!({ "t": "Slice", "bits": usize::BITS, "v": [] }),
            json!({ "t": "Vec", "bits": 128, "v": [] }),
            json!({ "t": "Vec", "bits": usize::BITS / 2, "v": [] }),
            json!({ "t": "Vec", "v": [] }),
            json!({ "t": "Vec", "bits": usize::BITS }),
            json!({ "t": "Vec", "bits": usize::BITS, "v": null }),
            json!({ "t": "Vec", "bits": usize::BITS, "v": [], "extra": true }),
            json!({ "t": "Vec", "bits": usize::BITS, "v": [42] }),
            json!({ "t": "Vec", "bits": usize::BITS, "v": [
                { "t": "u8", "bits": 8, "v": "256" }
            ] }),
        ] {
            assert!(serde_json::from_value::<VecSurrogate<u8>>(wire).is_err());
        }
    }

    #[test]
    fn methods_borrow_elements_and_clone_owned_storage() {
        let surrogate = vec![String::from("hello"), String::from("world")].into_surrogate();
        assert_eq!(surrogate.len().into_real(), 2);
        assert!(!surrogate.is_empty().into_real());
        let first = surrogate.first().into_real().unwrap();
        assert!(std::ptr::eq(first, surrogate.0.first().unwrap()));
        assert_eq!(surrogate.last().into_real(), Some(&surrogate.0[1]));
        assert!(surrogate.get(2usize.into_surrogate()).into_real().is_none());
        assert!(
            surrogate
                .get(usize::MAX.into_surrogate())
                .into_real()
                .is_none()
        );
        let mut copy = surrogate.as_slice().to_owned().into_real();
        copy[0].push('!');
        assert_eq!(first, "hello");
        let mut values = vec![42u8];
        assert_eq!(values.as_mut_slice().into_surrogate().len().into_real(), 1);
    }
}
