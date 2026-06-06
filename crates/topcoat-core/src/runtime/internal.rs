/// Converts a borrowed memoized `Option` or `Result` into the ergonomic
/// borrowed shape exposed by `#[memoize]`.
///
/// `#[memoize]` stores the function's original return value in the request
/// cache and normally returns `&T`. For top-level `Option<T>` and
/// `Result<T, E>` return values, the macro applies `.as_ref()` and publishes
/// this associated type instead, yielding `Option<&T>` and `Result<&T, &E>`.
#[doc(hidden)]
pub trait MemoizeAsRef {
    /// The return type produced after borrowing the cached value's contents.
    type AsRef;
}

impl<'a, T> MemoizeAsRef for &'a Option<T> {
    type AsRef = Option<&'a T>;
}

impl<'a, T, E> MemoizeAsRef for &'a Result<T, E> {
    type AsRef = Result<&'a T, &'a E>;
}
