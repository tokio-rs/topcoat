use std::{
    fmt::{Debug, Display},
    ops::Deref,
    sync::Arc,
};

pub type Result<T = (), E = Error> = ::core::result::Result<T, E>;

/// Error type used by Topcoat APIs.
///
/// This is a thin wrapper around [`anyhow::Error`] that provides a shared
/// application error type while still allowing callers to inspect and
/// downcast the underlying error when needed. Cloning is cheap: clones share
/// the underlying error, so the same error can travel along several paths.
#[derive(Debug, Clone)]
pub struct Error(Arc<anyhow::Error>);

impl Error {
    /// Attempt to downcast the error object to a concrete type.
    ///
    /// # Errors
    ///
    /// Returns `Err(Self)` if the stored error is not an instance of `E`,
    /// handing back the original error unchanged. A clone shares its
    /// contents, so downcasting an error with a live clone fails even when
    /// the stored error is an instance of `E`; drop the clones first.
    #[inline]
    pub fn downcast<E>(self) -> Result<E, Self>
    where
        E: Display + Debug + Send + Sync + 'static,
    {
        match Arc::try_unwrap(self.0) {
            Ok(error) => error.downcast::<E>().map_err(|error| Self(Arc::new(error))),
            Err(shared) => Err(Self(shared)),
        }
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

impl<T> From<T> for Error
where
    T: Into<anyhow::Error>,
{
    fn from(value: T) -> Self {
        Self(Arc::new(value.into()))
    }
}

impl From<Error> for Box<dyn std::error::Error + Send + Sync + 'static> {
    fn from(error: Error) -> Self {
        match Arc::try_unwrap(error.0) {
            Ok(error) => error.into(),
            Err(error) => Box::new(SharedError(error)),
        }
    }
}

/// A shared [`Error`]'s contents, boxed when the anyhow error it wraps
/// cannot be moved out of the share it lives in.
struct SharedError(Arc<anyhow::Error>);

impl Display for SharedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Debug for SharedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl std::error::Error for SharedError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        let inner: &(dyn std::error::Error + Send + Sync + 'static) = &**self.0;
        inner.source()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A payload type to recover from wrapped errors.
    #[derive(Debug)]
    struct Failure(&'static str);

    impl Display for Failure {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(self.0)
        }
    }

    impl std::error::Error for Failure {}

    #[test]
    fn clone_shares_the_underlying_error() {
        let error = Error::from(Failure("boom"));
        let clone = error.clone();

        assert_eq!(error.to_string(), "boom");
        assert_eq!(clone.to_string(), "boom");
        assert_eq!(
            clone.downcast_ref::<Failure>().map(|failure| failure.0),
            Some("boom")
        );
    }

    #[test]
    fn downcast_extracts_a_unique_error() {
        let failure = Error::from(Failure("boom")).downcast::<Failure>().unwrap();
        assert_eq!(failure.0, "boom");
    }

    #[test]
    fn downcast_keeps_a_non_matching_error() {
        let error = Error::from(std::io::Error::other("boom"));
        let error = error.downcast::<Failure>().unwrap_err();
        assert_eq!(error.to_string(), "boom");
    }

    #[test]
    fn downcast_fails_while_the_error_is_shared() {
        let error = Error::from(Failure("boom"));
        let clone = error.clone();

        let error = error.downcast::<Failure>().unwrap_err();
        assert_eq!(error.to_string(), "boom");

        // Dropping the share makes the error unique again.
        drop(clone);
        let failure = error.downcast::<Failure>().unwrap();
        assert_eq!(failure.0, "boom");
    }

    #[test]
    fn box_conversion_keeps_the_message() {
        let boxed: Box<dyn std::error::Error + Send + Sync> = Error::from(Failure("boom")).into();
        assert_eq!(boxed.to_string(), "boom");
    }

    #[test]
    fn box_conversion_of_a_shared_error_keeps_the_message() {
        let error = Error::from(Failure("boom"));
        let shared = error.clone();
        let boxed: Box<dyn std::error::Error + Send + Sync> = error.into();
        assert_eq!(boxed.to_string(), "boom");
        drop(shared);
    }

    #[test]
    fn box_conversion_of_a_shared_error_keeps_the_source() {
        #[derive(Debug)]
        struct WithCause {
            cause: Failure,
        }

        impl Display for WithCause {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("outer")
            }
        }

        impl std::error::Error for WithCause {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                Some(&self.cause)
            }
        }

        let error = Error::from(WithCause {
            cause: Failure("boom"),
        });
        let shared = error.clone();
        let boxed: Box<dyn std::error::Error + Send + Sync> = error.into();
        assert_eq!(boxed.to_string(), "outer");
        assert_eq!(
            boxed.source().map(ToString::to_string),
            Some(String::from("boom"))
        );
        drop(shared);
    }
}
