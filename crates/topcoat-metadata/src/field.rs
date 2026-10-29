mod image;
mod text;
mod url;

pub use image::*;
pub use text::*;
pub use url::*;

/// Whether a metadata field is unset, has a value, or has been removed.
#[derive(Default)]
pub(crate) enum Field<T> {
    /// Uses the value from a parent context, if there is one.
    #[default]
    Inherit,
    /// Uses this value for the field.
    Set(T),
    /// Omits the field, even if a parent context supplies a value.
    Removed,
}

impl<T> Field<T> {
    /// Copies the state of `field` into this field if it still inherits.
    pub(crate) fn fill(&mut self, field: Field<T>) {
        if let Self::Inherit = self {
            *self = field;
        }
    }

    /// Returns the stored value, or `None` for an unset or removed field.
    pub(crate) fn value(&self) -> Option<&T> {
        match self {
            Self::Set(value) => Some(value),
            Self::Inherit | Self::Removed => None,
        }
    }

    /// Returns the stored value, `fallback` for an unset field, or `None`
    /// for a removed field.
    pub(crate) fn value_or<'a>(&'a self, fallback: Option<&'a T>) -> Option<&'a T> {
        match self {
            Self::Set(value) => Some(value),
            Self::Inherit => fallback,
            Self::Removed => None,
        }
    }

    /// Applies `f` to a stored value. Unset and removed fields stay as they are.
    pub(crate) fn map<U>(self, f: impl FnOnce(T) -> U) -> Field<U> {
        match self {
            Self::Inherit => Field::Inherit,
            Self::Set(value) => Field::Set(f(value)),
            Self::Removed => Field::Removed,
        }
    }

    /// Borrows a stored value without changing whether the field is set.
    pub(crate) fn as_ref(&self) -> Field<&T> {
        match self {
            Self::Inherit => Field::Inherit,
            Self::Set(value) => Field::Set(value),
            Self::Removed => Field::Removed,
        }
    }
}

/// Copies fields from another value when they are still unset.
/// Combine metadata from the closest context first so its fields take priority.
pub(crate) trait Fill<From = Self> {
    /// Clones values from borrowed metadata to fill unset fields.
    fn fill_from(&mut self, from: &From);

    /// Moves values from owned metadata to fill unset fields.
    fn fill_from_owned(&mut self, from: From);
}

impl<T: Clone> Fill for Field<T> {
    fn fill_from(&mut self, from: &Self) {
        self.fill(from.as_ref().map(T::clone));
    }

    fn fill_from_owned(&mut self, from: Self) {
        self.fill(from);
    }
}

/// Fills unset fields from a group when that group is present.
impl<T: Fill> Fill<Option<Box<T>>> for T {
    fn fill_from(&mut self, from: &Option<Box<T>>) {
        if let Some(from) = from {
            <T as Fill>::fill_from(self, from);
        }
    }

    fn fill_from_owned(&mut self, from: Option<Box<T>>) {
        if let Some(from) = from {
            <T as Fill>::fill_from_owned(self, *from);
        }
    }
}

/// Implements [`Fill`] by matching the listed fields by name.
/// Use `Target from Source { ... }` when the source has a different type.
macro_rules! fill_fields {
    ($ty:ident from $from:ident { $($field:ident),* $(,)? }) => {
        impl $crate::field::Fill<$from> for $ty {
            fn fill_from(&mut self, from: &$from) {
                $($crate::field::Fill::fill_from(&mut self.$field, &from.$field);)*
            }

            fn fill_from_owned(&mut self, from: $from) {
                $($crate::field::Fill::fill_from_owned(&mut self.$field, from.$field);)*
            }
        }
    };
    ($ty:ident { $($field:ident),* $(,)? }) => {
        $crate::field::fill_fields!($ty from $ty { $($field),* });
    };
}

pub(crate) use fill_fields;
