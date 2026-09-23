/// Borrows the contents of a value returned by a `#[memoize(as_ref)]`
/// function.
///
/// `#[memoize]` stores the function's return value in the request cache and
/// returns `&T`. With `as_ref`, the macro instead borrows the cached value's
/// contents through this trait, so an `Option<T>` comes back as `Option<&T>`
/// and a `Result<T, E>` as `Result<&T, &E>`.
///
/// Implement this trait for your own return type to use it with
/// `#[memoize(as_ref)]`:
///
/// ```rust
/// use topcoat::context::MemoizeAsRef;
///
/// /// A custom optional value.
/// enum Maybe<T> {
///     Some(T),
///     None,
/// }
///
/// impl<T> MemoizeAsRef for Maybe<T> {
///     type AsRef<'a>
///         = Maybe<&'a T>
///     where
///         Self: 'a;
///
///     fn as_ref(&self) -> Self::AsRef<'_> {
///         match self {
///             Maybe::Some(value) => Maybe::Some(value),
///             Maybe::None => Maybe::None,
///         }
///     }
/// }
/// ```
pub trait MemoizeAsRef {
    /// The type returned when borrowing the cached value's contents.
    type AsRef<'a>
    where
        Self: 'a;

    /// Borrows the cached value's contents.
    fn as_ref(&self) -> Self::AsRef<'_>;
}

impl<T> MemoizeAsRef for Option<T> {
    type AsRef<'a>
        = Option<&'a T>
    where
        Self: 'a;

    fn as_ref(&self) -> Self::AsRef<'_> {
        Option::as_ref(self)
    }
}

impl<T, E> MemoizeAsRef for Result<T, E> {
    type AsRef<'a>
        = Result<&'a T, &'a E>
    where
        Self: 'a;

    fn as_ref(&self) -> Self::AsRef<'_> {
        Result::as_ref(self)
    }
}
