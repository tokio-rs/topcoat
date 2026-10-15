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
    /// Iterates over borrowed elements without cloning them.
    ///
    /// The iterator supports [`position`](SliceIter::position) and cannot be
    /// captured, serialized, or stored in a signal.
    #[must_use]
    #[allow(
        clippy::iter_not_returning_iterator,
        reason = "the expression vocabulary exposes only position on this cursor"
    )]
    pub fn iter(&self) -> SliceIter<'_, T> {
        SliceIter(self.0.iter())
    }

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

    /// Borrows the element at `index`.
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

/// A borrowed sequence cursor supporting [`position`](Self::position).
///
/// This is an intermediate expression value, not a serializable runtime value.
pub struct SliceIter<'a, T>(std::slice::Iter<'a, T>);

impl<'a, T> SliceIter<'a, T>
where
    &'a T: Surrogated,
{
    /// Returns the first matching position in the remaining elements.
    ///
    /// Visits elements in order and stops at the first match. Consumes every
    /// visited element, including the match. Each call counts from zero at the
    /// current cursor. Returns `None` when no remaining element matches.
    ///
    /// # Panics
    ///
    /// Propagates predicate panics. The failing element is consumed too.
    pub fn position<F>(&mut self, mut predicate: F) -> OptionSurrogate<usize>
    where
        F: FnMut(<&'a T as Surrogated>::Surrogate) -> BoolSurrogate,
    {
        OptionSurrogate::new(
            self.0
                .position(|value| predicate(value.into_surrogate()).into_real()),
        )
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

#[cfg(test)]
mod tests {
    use std::panic::{AssertUnwindSafe, catch_unwind};

    use super::*;

    #[test]
    fn position_matches_native_slices_and_consumption() {
        let values = [99u32, 10, 20, 20, 30, 99];
        for slice in [&values[..0], &values[1..5], &values[..]] {
            for target in [10, 20, 30, 99, 100] {
                let mut native = slice.iter();
                let mut borrowed = slice.into_surrogate().iter();
                for _ in 0..slice.len() + 2 {
                    let mut expected_visits = Vec::new();
                    let expected = native.position(|value| {
                        expected_visits.push(*value);
                        *value == target
                    });
                    let mut visits = Vec::new();
                    let actual = borrowed.position(|value| {
                        visits.push(*value.into_real());
                        value.eq(&target.into_surrogate())
                    });
                    assert_eq!(actual.into_real(), expected);
                    assert_eq!(visits, expected_visits);
                }
            }
        }
    }

    #[test]
    fn predicate_borrows_the_original_element_beyond_the_cursor_call() {
        let values = ["first".to_owned(), "last".to_owned()];
        let slice = values.as_slice().into_surrogate();
        let mut iter = slice.iter();
        let mut first = None;
        assert_eq!(
            iter.position(|value| {
                first = Some(value.into_real());
                true.into_surrogate()
            })
            .into_real(),
            Some(0)
        );
        assert_eq!(
            iter.position(|_value| true.into_surrogate()).into_real(),
            Some(0)
        );
        assert!(std::ptr::eq(first.unwrap(), values.first().unwrap()));
        assert_eq!(
            slice
                .iter()
                .position(|value| value.eq(values[0].as_str().into_surrogate()))
                .into_real(),
            Some(0)
        );
    }

    #[test]
    fn predicate_panic_consumes_the_failing_element() {
        let values = [10u32, 20, 30];
        let mut native = values.iter();
        let mut iter = values.as_slice().into_surrogate().iter();
        let mut visited = Vec::new();
        assert!(
            catch_unwind(AssertUnwindSafe(|| native.position(|value| {
                assert_ne!(*value, 20);
                false
            })))
            .is_err()
        );
        assert!(
            catch_unwind(AssertUnwindSafe(|| iter.position(|value| {
                let value = *value.into_real();
                visited.push(value);
                assert_ne!(value, 20);
                false.into_surrogate()
            })))
            .is_err()
        );
        assert_eq!(visited, [10, 20]);
        let remaining = iter.position(|value| {
            visited.push(*value.into_real());
            value.eq(&30u32.into_surrogate())
        });
        assert_eq!(remaining.into_real(), native.position(|value| *value == 30));
        assert_eq!(visited, [10, 20, 30]);
    }
}
