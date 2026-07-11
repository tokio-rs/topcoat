use std::{
    fmt::{Debug, Display},
    ops::{Deref, DerefMut},
};

pub type Result<T, E = Error> = ::core::result::Result<T, E>;

/// Error type used by Topcoat APIs.
///
/// This is a thin wrapper around [`anyhow::Error`] that provides a shared
/// application error type while still allowing callers to inspect, downcast,
/// and mutate the underlying error when needed.
#[derive(Debug)]
pub struct Error(anyhow::Error);

impl Error {
    /// Attempt to downcast the error object to a concrete type.
    ///
    /// # Errors
    ///
    /// Returns `Err(Self)` if the stored error is not an instance of `E`,
    /// handing back the original error unchanged.
    #[inline]
    pub fn downcast<E>(self) -> Result<E, Self>
    where
        E: Display + Debug + Send + Sync + 'static,
    {
        self.0.downcast::<E>().map_err(Self)
    }

    /// Downcast this error object by reference.
    #[inline]
    #[must_use]
    pub fn downcast_ref<E>(&self) -> Option<&E>
    where
        E: Display + Debug + Send + Sync + 'static,
    {
        self.0.downcast_ref::<E>()
    }

    /// Downcast this error object by mutable reference.
    #[inline]
    pub fn downcast_mut<E>(&mut self) -> Option<&mut E>
    where
        E: Display + Debug + Send + Sync + 'static,
    {
        self.0.downcast_mut::<E>()
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Deref for Error {
    type Target = anyhow::Error;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Error {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> From<T> for Error
where
    T: Into<anyhow::Error>,
{
    fn from(value: T) -> Self {
        Self(value.into())
    }
}
