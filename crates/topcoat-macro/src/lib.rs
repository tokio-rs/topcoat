mod component;
mod layout;
mod memoize;
mod page;
mod quote_option;
mod route;
mod segment;

use proc_macro::TokenStream;
use quote::quote;
use topcoat_view::ast::View;

#[proc_macro]
pub fn view(tokens: TokenStream) -> TokenStream {
    let parsed = syn::parse_macro_input!(tokens as View);
    quote! { #parsed }.into()
}

#[proc_macro_attribute]
pub fn component(attr: TokenStream, item: TokenStream) -> TokenStream {
    let _attr = syn::parse_macro_input!(attr as component::ComponentAttr);
    let item = syn::parse_macro_input!(item as component::ComponentItem);
    quote! { #item }.into()
}

#[proc_macro_attribute]
pub fn route(attr: TokenStream, item: TokenStream) -> TokenStream {
    let _attr = syn::parse_macro_input!(attr as route::RouteAttr);
    let item = syn::parse_macro_input!(item as route::RouteItem);
    quote! { #item }.into()
}

#[proc_macro_attribute]
pub fn page(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = syn::parse_macro_input!(attr as page::PageAttr);
    let item = syn::parse_macro_input!(item as page::PageItem);
    let page = page::Page::new(attr, item);
    quote! { #page }.into()
}

#[proc_macro_attribute]
pub fn layout(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = syn::parse_macro_input!(attr as layout::LayoutAttr);
    let item = syn::parse_macro_input!(item as layout::LayoutItem);
    let layout = layout::Layout::new(attr, item);
    quote! { #layout }.into()
}

#[proc_macro]
pub fn segment(tokens: TokenStream) -> TokenStream {
    let segment = syn::parse_macro_input!(tokens as segment::Segment);
    quote! { #segment }.into()
}

/// Caches the result of a function for the duration of a request, keyed by its arguments.
///
/// The annotated function must take a `cx: &Cx` parameter as its handle into the request
/// context. All other arguments form the cache key: the first call with a given set of
/// arguments runs the body and stores the result; subsequent calls with equal arguments
/// return the cached value without re-running the body.
///
/// The function's return type `T` is rewritten to [`Memoized<'_, T>`], a handle that
/// dereferences to `&T` and is bound to the lifetime of the request context.
///
/// # Sync and async
///
/// `#[memoize]` works on both synchronous and `async` functions. Async functions are
/// memoized such that concurrent callers with the same arguments share a single in-flight
/// future and observe the same result.
///
/// ```ignore
/// use topcoat::{context::Cx, memoize};
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
///
/// [`Memoized<'_, T>`]: topcoat::context::Memoized
#[proc_macro_attribute]
pub fn memoize(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = syn::parse_macro_input!(attr as memoize::MemoizeAttr);
    let item = syn::parse_macro_input!(item as memoize::MemoizeItem);
    let memoize = memoize::Memoize::new(attr, item);
    quote! { #memoize }.into()
}
