use proc_macro::TokenStream;
use quote::quote;

/// Caches the result of a function for the duration of a request, keyed by its arguments.
///
/// The annotated function must take a `cx: &Cx` parameter as its handle into the request
/// context. All other arguments form the cache key: the first call with a given set of
/// arguments runs the body and stores the result; subsequent calls with equal arguments
/// return the cached value without re-running the body.
///
/// The function's return type `T` is rewritten to `&T` that has the same lifetime as `&cx`.
/// Top-level `Option<T>` and `Result<T, E>` return types instead become `Option<&T>` and
/// `Result<&T, &E>`, matching the standard `.as_ref()` borrowing shape.
///
/// # Sync and async
///
/// `#[memoize]` works on both synchronous and `async` functions. Async functions are
/// memoized such that concurrent callers with the same arguments share a single in-flight
/// future and observe the same result.
///
/// ```ignore
/// use topcoat::context::{Cx, memoize};
///
/// // Synchronous: the body runs once per `(x, y)` pair.
/// #[memoize]
/// fn add(cx: &Cx, x: i32, y: i32) -> i32 {
///     x + y
/// }
///
/// // Asynchronous: borrowed arguments like `&str` are accepted and stored as owned keys.
/// #[memoize]
/// async fn fetch_user(cx: &Cx, id: &str) -> User {
///     db::load_user(id).await
/// }
///
/// async fn handler(cx: &Cx) {
///     let sum = add(cx, 2, 3); // computes
///     let sum = add(cx, 2, 3); // cached
///     let user = fetch_user(cx, "alice").await; // computes
///     let user = fetch_user(cx, "alice").await; // cached
/// }
/// ```
///
/// # Requirements
///
/// - The function must accept a parameter named `cx` of type `&Cx`.
/// - The function cannot take a `self` receiver.
/// - Each non-`cx` argument is part of the cache key. For an owned argument of type `T`,
///   `T` must be `Clone + Hash + Eq + Send + Sync + 'static`. For a borrowed argument of
///   type `&Q`, `Q` must be `ToOwned` with `Q::Owned: Hash + Eq + Send + Sync + 'static`
///   (e.g. `&str` works because `String: Hash + Eq + Send + Sync + 'static`).
/// - The return type `T` must be `Send + Sync + 'static`.
#[proc_macro_attribute]
pub fn memoize(attr: TokenStream, item: TokenStream) -> TokenStream {
    match topcoat_core::ast::memoize::Memoize::parse(attr.into(), item.into()) {
        Ok(value) => quote! { #value }.into(),
        Err(error) => error.to_compile_error().into(),
    }
}
