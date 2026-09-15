use ref_cast::RefCast;
use serde::ser::{SerializeSeq, SerializeStruct};

use crate::{
    BoolSurrogate, OptionSurrogate, Surrogate, Surrogated, UsizeSurrogate, VecSurrogate,
    impl_surrogate_mut, impl_surrogate_ref,
};

/// A borrowed sequence of runtime values.
#[derive(Debug, RefCast)]
#[repr(transparent)]
pub struct SliceSurrogate<T>([T]);

impl<T> SliceSurrogate<T> {
    #[must_use]
    pub fn len(&self) -> UsizeSurrogate {
        UsizeSurrogate::new(self.0.len())
    }

    #[must_use]
    pub fn is_empty(&self) -> BoolSurrogate {
        BoolSurrogate::new(self.0.is_empty())
    }

    #[must_use]
    pub fn get(&self, index: UsizeSurrogate) -> OptionSurrogate<&T> {
        OptionSurrogate::new(self.0.get(index.into_real()))
    }

    #[must_use]
    pub fn first(&self) -> OptionSurrogate<&T> {
        OptionSurrogate::new(self.0.first())
    }

    #[must_use]
    pub fn last(&self) -> OptionSurrogate<&T> {
        OptionSurrogate::new(self.0.last())
    }
}

impl<T: Clone> SliceSurrogate<T> {
    #[must_use]
    pub fn to_vec(&self) -> VecSurrogate<T> {
        VecSurrogate::new(self.0.to_vec())
    }

    #[must_use]
    pub fn to_owned(&self) -> VecSurrogate<T> {
        self.to_vec()
    }
}

impl<T> SliceSurrogate<T>
where
    for<'a> &'a T: Surrogated,
    for<'a> <&'a T as Surrogated>::Surrogate: serde::Serialize,
{
    pub(super) fn serialize_with_tag<S>(
        &self,
        serializer: S,
        tag: &'static str,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut value = serializer.serialize_struct("Sequence", 3)?;
        value.serialize_field("t", tag)?;
        value.serialize_field("bits", &usize::BITS)?;
        value.serialize_field("v", &Elements(&self.0))?;
        value.end()
    }
}

impl_surrogate_ref!({ T }[T], SliceSurrogate<T>);
impl_surrogate_mut!({ T }[T], SliceSurrogate<T>);

impl<T> serde::Serialize for SliceSurrogate<T>
where
    for<'a> &'a T: Surrogated,
    for<'a> <&'a T as Surrogated>::Surrogate: serde::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.serialize_with_tag(serializer, "Slice")
    }
}

struct Elements<'a, T>(&'a [T]);

impl<T> serde::Serialize for Elements<'_, T>
where
    for<'a> &'a T: Surrogated,
    for<'a> <&'a T as Surrogated>::Surrogate: serde::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for element in self.0 {
            sequence.serialize_element(&element.into_surrogate())?;
        }
        sequence.end()
    }
}
