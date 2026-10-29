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
