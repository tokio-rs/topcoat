mod image;
mod text;
mod url;

pub use image::*;
pub use text::*;
pub use url::*;

/// The state of one field in one level of metadata.
#[derive(Default)]
pub(crate) enum Field<T> {
    /// The field is not set here, so a level further up decides.
    #[default]
    Inherit,
    /// The field is set to this value.
    Set(T),
    /// The field is removed, hiding any value set further up.
    Removed,
}

impl<T> Field<T> {
    /// Replaces this field with `field` if it is not set or removed yet.
    pub(crate) fn fill(&mut self, field: Field<T>) {
        if let Self::Inherit = self {
            *self = field;
        }
    }

    /// Returns the value if the field is set.
    pub(crate) fn value(&self) -> Option<&T> {
        match self {
            Self::Set(value) => Some(value),
            Self::Inherit | Self::Removed => None,
        }
    }

    /// Returns the value if the field is set, or `fallback` if no level set
    /// or removed it.
    pub(crate) fn value_or<'a>(&'a self, fallback: Option<&'a T>) -> Option<&'a T> {
        match self {
            Self::Set(value) => Some(value),
            Self::Inherit => fallback,
            Self::Removed => None,
        }
    }

    /// Converts the value with `f`, keeping the state.
    pub(crate) fn map<U>(self, f: impl FnOnce(T) -> U) -> Field<U> {
        match self {
            Self::Inherit => Field::Inherit,
            Self::Set(value) => Field::Set(f(value)),
            Self::Removed => Field::Removed,
        }
    }

    /// Borrows the value, keeping the state.
    pub(crate) fn as_ref(&self) -> Field<&T> {
        match self {
            Self::Inherit => Field::Inherit,
            Self::Set(value) => Field::Set(value),
            Self::Removed => Field::Removed,
        }
    }
}

/// Fills the fields of a value that are still unset from the fields of
/// another one, combining levels of metadata from the closest one out.
pub(crate) trait Fill<From = Self> {
    /// Fills from metadata stored in the context, cloning its values.
    fn fill_from(&mut self, from: &From);

    /// Fills from metadata loaded for the request, taking its values.
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

/// Fills a group of fields from a level that may not set any of them.
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

/// Implements [`Fill`] for a struct, filling each listed field from the
/// field of the same name. `Target from Source { ... }` fills from another
/// type.
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
