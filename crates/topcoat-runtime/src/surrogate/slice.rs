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
    /// Returns the number of elements.
    #[must_use]
    pub fn len(&self) -> UsizeSurrogate {
        UsizeSurrogate::new(self.0.len())
    }

    /// Returns whether the slice has no elements.
    #[must_use]
    pub fn is_empty(&self) -> BoolSurrogate {
        BoolSurrogate::new(self.0.is_empty())
    }

    /// Borrows the element at `index`, or returns `None` if `index` is out
    /// of bounds.
    #[must_use]
    pub fn get(&self, index: UsizeSurrogate) -> OptionSurrogate<&T> {
        OptionSurrogate::new(self.0.get(index.into_real()))
    }

    /// Borrows the element at `index`. Indexing with `slice[index]` calls
    /// this.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of bounds.
    #[must_use]
    #[track_caller]
    pub fn index<'a>(&'a self, index: UsizeSurrogate) -> <&'a T as Surrogated>::Surrogate
    where
        &'a T: Surrogated,
    {
        (&self.0[index.into_real()]).into_surrogate()
    }

    /// Borrows the first element, or returns `None` if the slice is empty.
    #[must_use]
    pub fn first(&self) -> OptionSurrogate<&T> {
        OptionSurrogate::new(self.0.first())
    }

    /// Borrows the last element, or returns `None` if the slice is empty.
    #[must_use]
    pub fn last(&self) -> OptionSurrogate<&T> {
        OptionSurrogate::new(self.0.last())
    }
}

impl<T: Clone> SliceSurrogate<T> {
    /// Copies the elements into a new vector.
    #[must_use]
    pub fn to_vec(&self) -> VecSurrogate<T> {
        VecSurrogate::new(self.0.to_vec())
    }

    /// Copies the elements into a new vector, like [`to_vec`](Self::to_vec).
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
