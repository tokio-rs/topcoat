use std::{
    any::Any,
    backtrace::{Backtrace, BacktraceStatus},
    fmt::{self, Debug, Display},
    sync::Arc,
};

pub type Result<T = (), E = Error> = ::core::result::Result<T, E>;

/// Error type used by Topcoat APIs.
///
/// Use `?` to convert an [`std::error::Error`] that is `Send + Sync + 'static`.
/// Inspect the original error with [`downcast_ref`](Self::downcast_ref), or
/// recover it with [`downcast`](Self::downcast). Clones share the error.
/// Backtrace capture follows `RUST_BACKTRACE` and `RUST_LIB_BACKTRACE`.
#[derive(Clone)]
pub struct Error(Arc<dyn ErrorObject>);

impl Error {
    /// Builds an error from a message.
    pub fn msg(message: impl Display + Debug + Send + Sync + 'static) -> Self {
        Self::from(Message(message))
    }

    /// Builds an error from a boxed error. Its message and sources carry
    /// over, but the concrete error inside the box is not reachable through
    /// the downcast methods.
    #[must_use]
    pub fn from_boxed(error: Box<dyn std::error::Error + Send + Sync + 'static>) -> Self {
        Self::from(Boxed(error))
    }

    /// Builds an error from an [`anyhow::Error`]. Its message and sources
    /// carry over, but the concrete error inside it is not reachable through
    /// the downcast methods.
    #[cfg(feature = "anyhow")]
    #[must_use]
    pub fn from_anyhow(error: anyhow::Error) -> Self {
        Self::from_boxed(error.into())
    }

    /// Wraps this error in a context message, which becomes its [`Display`]
    /// output. The wrapped error stays reachable through
    /// [`source`](std::error::Error::source), [`chain`](Self::chain), and
    /// the downcast methods.
    #[must_use]
    pub fn context(self, context: impl Display + Send + Sync + 'static) -> Self {
        Self::from(WithContext {
            context: Box::new(context),
            error: self,
        })
    }

    /// Whether the stored error is an instance of `E`, looking through
    /// [`context`](Self::context) layers.
    #[inline]
    #[must_use]
    pub fn is<E>(&self) -> bool
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        self.downcast_ref::<E>().is_some()
    }

    /// Attempt to move the concrete error out of this error object, looking
    /// through [`context`](Self::context) layers and discarding their
    /// messages.
    ///
    /// This never clones the stored error, so it only succeeds while this is
    /// the sole handle to it. To fall back to a clone instead, use
    /// [`downcast_cloned`](Self::downcast_cloned).
    ///
    /// # Errors
    ///
    /// Returns a [`DowncastError`] carrying the original error back, and
    /// saying whether the stored error is not an instance of `E` or a clone
    /// of it is still alive.
    pub fn downcast<E>(self) -> Result<E, DowncastError>
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        if !self.is::<E>() {
            return Err(DowncastError::new(DowncastFailure::Mismatch, self));
        }
        self.unwrap::<E>()
    }

    /// Attempt to downcast the error object to a concrete type, looking
    /// through [`context`](Self::context) layers.
    ///
    /// The stored error is moved out when this is the sole handle to it and
    /// cloned otherwise.
    ///
    /// # Errors
    ///
    /// Returns `Err(Self)` if the stored error is not an instance of `E`,
    /// handing back the original error unchanged.
    pub fn downcast_cloned<E>(self) -> Result<E, Self>
    where
        E: std::error::Error + Send + Sync + Clone + 'static,
    {
        match self.downcast::<E>() {
            Ok(error) => Ok(error),
            Err(failed) => {
                let error = failed.into_error();
                match error.downcast_ref::<E>() {
                    Some(error) => Ok(error.clone()),
                    None => Err(error),
                }
            }
        }
    }

    /// Downcast this error object by reference, looking through
    /// [`context`](Self::context) layers.
    #[must_use]
    pub fn downcast_ref<E>(&self) -> Option<&E>
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        let error = self.0.error();
        match error.downcast_ref::<E>() {
            Some(error) => Some(error),
            None => error
                .downcast_ref::<WithContext>()
                .and_then(|context| context.error.downcast_ref::<E>()),
        }
    }

    /// Downcast this error object by mutable reference, looking through
    /// [`context`](Self::context) layers.
    ///
    /// # Errors
    ///
    /// Returns a [`DowncastFailure`] saying whether the stored error is not
    /// an instance of `E` or a clone of it is still alive: shared contents
    /// cannot be handed out mutably.
    pub fn downcast_mut<E>(&mut self) -> Result<&mut E, DowncastFailure>
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        if !self.is::<E>() {
            return Err(DowncastFailure::Mismatch);
        }
        self.unwrap_mut::<E>()
    }

    /// Moves the stored error out of an error known to hold an `E`, either
    /// directly or behind context layers.
    fn unwrap<E>(self) -> Result<E, DowncastError>
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        let any = match self.0.into_any().downcast::<Object<E>>() {
            Ok(object) => {
                return match Arc::try_unwrap(object) {
                    Ok(object) => Ok(object.error),
                    Err(shared) => Err(DowncastError::new(DowncastFailure::Shared, Self(shared))),
                };
            }
            Err(any) => any,
        };
        let Ok(object) = any.downcast::<Object<WithContext>>() else {
            unreachable!("an error holding an `E` is an `Object<E>` or a context layer");
        };
        match Arc::try_unwrap(object) {
            Ok(object) => {
                let WithContext { context, error } = object.error;
                error.unwrap::<E>().map_err(|failed| {
                    // Put the layer back so the original error is handed back
                    // intact, backtrace included.
                    let error = Self(Arc::new(Object {
                        error: WithContext {
                            context,
                            error: failed.error,
                        },
                        backtrace: object.backtrace,
                    }));
                    DowncastError::new(failed.failure, error)
                })
            }
            Err(shared) => Err(DowncastError::new(DowncastFailure::Shared, Self(shared))),
        }
    }

    /// Borrows the stored error of an error known to hold an `E`, either
    /// directly or behind context layers.
    fn unwrap_mut<E>(&mut self) -> Result<&mut E, DowncastFailure>
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        let error = Arc::get_mut(&mut self.0)
            .ok_or(DowncastFailure::Shared)?
            .error_mut();
        if error.is::<E>() {
            return Ok(error
                .downcast_mut::<E>()
                .expect("the stored error was checked to be an `E`"));
        }
        error
            .downcast_mut::<WithContext>()
            .expect("an error holding an `E` is an `E` or a context layer")
            .error
            .unwrap_mut::<E>()
    }

    /// The backtrace captured when the error was built. Capture follows the
    /// `RUST_BACKTRACE` and `RUST_LIB_BACKTRACE` environment variables; check
    /// [`Backtrace::status`] before relying on its contents.
    #[inline]
    pub fn backtrace(&self) -> &Backtrace {
        self.0.backtrace()
    }

    /// The stored error followed by each of its
    /// [`source`](std::error::Error::source)s, outermost first.
    pub fn chain(&self) -> impl Iterator<Item = &(dyn std::error::Error + 'static)> {
        let mut next: Option<&(dyn std::error::Error + 'static)> = Some(self.0.error());
        std::iter::from_fn(move || {
            let current = next?;
            next = current.source();
            Some(current)
        })
    }
}

/// Prints the stored error's message; the alternate form (`{:#}`) appends
/// each source after a colon.
impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(self.0.error(), f)?;
        if f.alternate() {
            for cause in self.chain().skip(1) {
                write!(f, ": {cause}")?;
            }
        }
        Ok(())
    }
}

/// Prints the stored error's message, its sources, and the backtrace when one
/// was captured; the alternate form (`{:#?}`) prints the stored error's own
/// [`Debug`] output instead.
impl Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            return Debug::fmt(self.0.error(), f);
        }
        write!(f, "{}", self.0.error())?;
        let mut causes = self.chain().skip(1).peekable();
        if causes.peek().is_some() {
            write!(f, "\n\nCaused by:")?;
            for cause in causes {
                write!(f, "\n    {cause}")?;
            }
        }
        let backtrace = self.backtrace();
        if backtrace.status() == BacktraceStatus::Captured {
            write!(f, "\n\nStack backtrace:\n{backtrace}")?;
        }
        Ok(())
    }
}

impl<E> From<E> for Error
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn from(error: E) -> Self {
        Self(Arc::new(Object {
            error,
            backtrace: Backtrace::capture(),
        }))
    }
}

impl From<Error> for Box<dyn std::error::Error + Send + Sync + 'static> {
    fn from(error: Error) -> Self {
        Box::new(BoxedError(error))
    }
}

/// Why a downcast could not hand out the stored error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DowncastFailure {
    /// The stored error is not an instance of the requested type.
    Mismatch,
    /// A clone of the error is still alive, so its contents can neither be
    /// moved out nor borrowed mutably.
    Shared,
}

impl Display for DowncastFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mismatch => f.write_str("the error is not an instance of the requested type"),
            Self::Shared => f.write_str("the error is still shared with a clone"),
        }
    }
}

impl std::error::Error for DowncastFailure {}

/// A failed [`Error::downcast`], carrying the original error back.
///
/// Use [`failure`](Self::failure) to inspect the reason and
/// [`into_error`](Self::into_error) to recover the original error.
#[derive(Debug)]
pub struct DowncastError {
    failure: DowncastFailure,
    error: Error,
}

impl DowncastError {
    fn new(failure: DowncastFailure, error: Error) -> Self {
        Self { failure, error }
    }

    /// Why the downcast failed.
    #[must_use]
    pub fn failure(&self) -> DowncastFailure {
        self.failure
    }

    /// The error the downcast was attempted on.
    #[must_use]
    pub fn error(&self) -> &Error {
        &self.error
    }

    /// Hands the original error back.
    #[must_use]
    pub fn into_error(self) -> Error {
        self.error
    }
}

/// The object an [`Error`] points at: a concrete error together with the
/// backtrace captured when it was wrapped.
///
/// A trait object rather than `dyn std::error::Error` so the error can be
/// handed back by value: [`into_any`](Self::into_any) recovers the concrete
/// object, which [`Arc::try_unwrap`] then moves out of.
trait ErrorObject: Send + Sync + 'static {
    /// The stored error.
    fn error(&self) -> &(dyn std::error::Error + Send + Sync + 'static);

    /// The stored error, mutably.
    fn error_mut(&mut self) -> &mut (dyn std::error::Error + Send + Sync + 'static);

    /// The backtrace captured when the error was wrapped.
    fn backtrace(&self) -> &Backtrace;

    /// Erases the object to `dyn Any`, from which its concrete type can be
    /// recovered with [`Arc::downcast`].
    fn into_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync>;
}

/// The concrete [`ErrorObject`] holding an error of type `E`.
struct Object<E> {
    error: E,
    backtrace: Backtrace,
}

impl<E> ErrorObject for Object<E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn error(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
        &self.error
    }

    fn error_mut(&mut self) -> &mut (dyn std::error::Error + Send + Sync + 'static) {
        &mut self.error
    }

    fn backtrace(&self) -> &Backtrace {
        &self.backtrace
    }

    fn into_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
        self
    }
}

/// The error behind [`Error::msg`].
struct Message<M>(M);

impl<M: Display> Display for Message<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl<M: Debug> Debug for Message<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl<M: Display + Debug> std::error::Error for Message<M> {}

/// The error behind [`Error::context`]: a message in front of the error it
/// wraps. Not generic over the message so a downcast can look through it.
struct WithContext {
    context: Box<dyn Display + Send + Sync>,
    error: Error,
}

impl Display for WithContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.context, f)
    }
}

impl Debug for WithContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WithContext")
            .field("context", &format_args!("{}", self.context))
            .field("error", &self.error)
            .finish()
    }
}

impl std::error::Error for WithContext {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.error.0.error())
    }
}

/// The error behind [`Error::from_boxed`], since a box of a trait object is
/// not itself an [`std::error::Error`].
struct Boxed(Box<dyn std::error::Error + Send + Sync + 'static>);

impl Display for Boxed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Debug for Boxed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl std::error::Error for Boxed {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

/// An [`Error`] in the shape of an [`std::error::Error`], for conversion
/// into a boxed error. [`Error`] cannot implement the trait itself: the
/// blanket conversion from every [`std::error::Error`] would then apply to
/// it and collide with the reflexive `From<Error> for Error`.
struct BoxedError(Error);

impl Display for BoxedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Debug for BoxedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl std::error::Error for BoxedError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.0.error().source()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A payload type to recover from wrapped errors.
    #[derive(Debug, Clone)]
    struct Failure(&'static str);

    impl Display for Failure {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(self.0)
        }
    }

    impl std::error::Error for Failure {}

    /// A payload type with a source.
    #[derive(Debug)]
    struct WithCause {
        cause: Failure,
    }

    impl Display for WithCause {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("outer")
        }
    }

    impl std::error::Error for WithCause {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            Some(&self.cause)
        }
    }

    #[test]
    fn the_error_is_two_words_wide() {
        assert_eq!(size_of::<Error>(), 2 * size_of::<usize>());
        assert_eq!(size_of::<Result<()>>(), 2 * size_of::<usize>());
    }

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
    fn msg_builds_an_error_from_a_message() {
        let error = Error::msg(format!("boom {}", 1));
        assert_eq!(error.to_string(), "boom 1");
        assert!(error.is::<Message<String>>());
    }

    #[test]
    fn downcast_extracts_a_unique_error() {
        let failure = Error::from(Failure("boom")).downcast::<Failure>().unwrap();
        assert_eq!(failure.0, "boom");
    }

    #[test]
    fn downcast_reports_a_mismatch() {
        let error = Error::from(std::io::Error::other("boom"));
        let failed = error.downcast::<Failure>().unwrap_err();
        assert_eq!(failed.failure(), DowncastFailure::Mismatch);
        assert_eq!(failed.into_error().to_string(), "boom");
    }

    #[test]
    fn downcast_reports_a_shared_error() {
        let error = Error::from(Failure("boom"));
        let clone = error.clone();

        let failed = error.downcast::<Failure>().unwrap_err();
        assert_eq!(failed.failure(), DowncastFailure::Shared);
        let error = failed.into_error();
        assert_eq!(error.to_string(), "boom");

        // Dropping the share makes the error unique again.
        drop(clone);
        let failure = error.downcast::<Failure>().unwrap();
        assert_eq!(failure.0, "boom");
    }

    #[test]
    fn downcast_moves_out_of_a_context_layer() {
        let error = Error::from(Failure("boom")).context("loading");
        let failure = error.downcast::<Failure>().unwrap();
        assert_eq!(failure.0, "boom");
    }

    #[test]
    fn downcast_hands_a_context_layer_back_intact() {
        let inner = Error::from(Failure("boom"));
        let shared = inner.clone();
        let error = inner.context("loading");

        let failed = error.downcast::<Failure>().unwrap_err();
        assert_eq!(failed.failure(), DowncastFailure::Shared);
        let error = failed.into_error();
        assert_eq!(format!("{error:#}"), "loading: boom");
        drop(shared);
    }

    #[test]
    fn downcast_cloned_extracts_a_unique_error() {
        let failure = Error::from(Failure("boom"))
            .downcast_cloned::<Failure>()
            .unwrap();
        assert_eq!(failure.0, "boom");
    }

    #[test]
    fn downcast_cloned_clones_a_shared_error() {
        let error = Error::from(Failure("boom"));
        let clone = error.clone();

        let failure = error.downcast_cloned::<Failure>().unwrap();
        assert_eq!(failure.0, "boom");
        assert_eq!(clone.to_string(), "boom");
    }

    #[test]
    fn downcast_cloned_keeps_a_non_matching_error() {
        let error = Error::from(std::io::Error::other("boom"));
        let clone = error.clone();

        let error = error.downcast_cloned::<Failure>().unwrap_err();
        assert_eq!(error.to_string(), "boom");
        drop(clone);
    }

    #[test]
    fn downcast_mut_mutates_a_unique_error() {
        let mut error = Error::from(Failure("boom"));
        error.downcast_mut::<Failure>().unwrap().0 = "bang";
        assert_eq!(error.to_string(), "bang");
    }

    #[test]
    fn downcast_mut_mutates_through_a_context_layer() {
        let mut error = Error::from(Failure("boom")).context("loading");
        error.downcast_mut::<Failure>().unwrap().0 = "bang";
        assert_eq!(format!("{error:#}"), "loading: bang");
    }

    #[test]
    fn downcast_mut_reports_a_mismatch() {
        let mut error = Error::from(std::io::Error::other("boom"));
        assert_eq!(
            error.downcast_mut::<Failure>().unwrap_err(),
            DowncastFailure::Mismatch
        );
    }

    #[test]
    fn downcast_mut_reports_a_shared_error() {
        let mut error = Error::from(Failure("boom"));
        let clone = error.clone();

        assert_eq!(
            error.downcast_mut::<Failure>().unwrap_err(),
            DowncastFailure::Shared
        );

        drop(clone);
        assert!(error.downcast_mut::<Failure>().is_ok());
    }

    #[test]
    fn context_wraps_the_message_and_keeps_the_error_reachable() {
        let error = Error::from(Failure("boom")).context("loading");

        assert_eq!(error.to_string(), "loading");
        assert_eq!(format!("{error:#}"), "loading: boom");
        assert_eq!(error.chain().count(), 2);
        assert!(error.is::<Failure>());
        assert_eq!(error.downcast_cloned::<Failure>().unwrap().0, "boom");
    }

    #[test]
    fn display_and_debug_walk_the_chain() {
        let error = Error::from(WithCause {
            cause: Failure("boom"),
        });

        assert_eq!(error.to_string(), "outer");
        assert_eq!(format!("{error:#}"), "outer: boom");
        assert!(format!("{error:?}").starts_with("outer\n\nCaused by:\n    boom"));
        assert!(format!("{error:#?}").starts_with("WithCause"));
    }

    #[test]
    fn box_conversion_keeps_the_message_and_source() {
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

    #[test]
    fn boxed_errors_convert_with_their_chain() {
        let boxed: Box<dyn std::error::Error + Send + Sync> = Box::new(WithCause {
            cause: Failure("boom"),
        });
        let error = Error::from_boxed(boxed);
        assert_eq!(error.to_string(), "outer");
        assert_eq!(format!("{error:#}"), "outer: boom");
    }

    #[cfg(feature = "anyhow")]
    #[test]
    fn anyhow_errors_convert_with_their_chain() {
        let error = Error::from_anyhow(anyhow::anyhow!("boom").context("loading"));
        assert_eq!(error.to_string(), "loading");
        assert_eq!(format!("{error:#}"), "loading: boom");
    }
}
