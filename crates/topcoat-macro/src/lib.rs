mod memoize;
mod quote_option;

#[cfg(feature = "view")]
mod component;

#[cfg(feature = "router")]
mod layout;
#[cfg(feature = "router")]
mod page;
#[cfg(feature = "router")]
mod path_param;
#[cfg(feature = "router")]
mod query_params;
#[cfg(feature = "router")]
mod route;
#[cfg(feature = "router")]
mod segment;

use proc_macro::TokenStream;
use quote::quote;

#[cfg(feature = "view")]
#[proc_macro]
pub fn view(tokens: TokenStream) -> TokenStream {
    let parsed = syn::parse_macro_input!(tokens as topcoat_view::ast::View);
    quote! { #parsed }.into()
}

#[cfg(feature = "view")]
#[proc_macro_attribute]
pub fn component(attr: TokenStream, item: TokenStream) -> TokenStream {
    let _attr = syn::parse_macro_input!(attr as component::ComponentAttr);
    let item = syn::parse_macro_input!(item as component::ComponentItem);
    quote! { #item }.into()
}

#[cfg(feature = "router")]
#[proc_macro_attribute]
pub fn route(attr: TokenStream, item: TokenStream) -> TokenStream {
    let _attr = syn::parse_macro_input!(attr as route::RouteAttr);
    let item = syn::parse_macro_input!(item as route::RouteItem);
    quote! { #item }.into()
}

#[cfg(feature = "router")]
#[proc_macro_attribute]
pub fn page(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = syn::parse_macro_input!(attr as page::PageAttr);
    let item = syn::parse_macro_input!(item as page::PageItem);
    let combined = page::Page::new(attr, item);
    quote! { #combined }.into()
}

#[cfg(feature = "router")]
#[proc_macro_attribute]
pub fn layout(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = syn::parse_macro_input!(attr as layout::LayoutAttr);
    let item = syn::parse_macro_input!(item as layout::LayoutItem);
    let combined = layout::Layout::new(attr, item);
    quote! { #combined }.into()
}

#[cfg(feature = "router")]
#[proc_macro]
pub fn segment(tokens: TokenStream) -> TokenStream {
    let segment = syn::parse_macro_input!(tokens as segment::Segment);
    quote! { #segment }.into()
}

/// Declares a typed view of a path parameter.
///
/// Apply this attribute to a tuple struct with a single unnamed field. The
/// struct name, snake-cased, becomes the parameter's name; the inner type
/// defines how the raw string is parsed.
///
/// # Pairing with the route's URL
///
/// `#[path_param]` only declares how to read a parameter — it does not by
/// itself decide which URL segment carries that parameter. How the param
/// gets into the URL depends on which router you use:
///
/// - **Module router** ([`module_router!`](../router/macro.module_router.html)) —
///   the macro also emits a `segment!(kind = Param, rename = "...")` for the
///   enclosing module. The module's URL segment is replaced by the
///   parameter, so a `PostId` defined anywhere in module `app::posts::id`
///   turns that module into `{post_id}` in the URL.
///
/// - **Regular [`Router`](../router/struct.Router.html)** — the page's path
///   string is the source of truth. Include a matching parameter name in
///   the `#[page("...")]` path; the snake-cased struct name must equal the
///   `{...}` placeholder for `of` to find the value. The `segment!` emitted
///   by the macro is inert for this router.
///
/// # Reading the parameter
///
/// The macro generates an `of(cx: &Cx)` associated function whose return
/// type depends on the inner type:
///
/// - **`&str`** — returns `&Self` directly with the borrowed segment value.
/// - **Any other type** — returns `&Result<Self, <T as FromStr>::Err>`,
///   parsed via [`FromStr`](core::str::FromStr). Parsing is memoized per
///   request, so repeated calls within a handler do not re-parse.
///
/// A [`Deref`](core::ops::Deref) impl to the inner type is also generated.
///
/// # Examples
///
/// ## Module router
///
/// ```ignore
/// // src/app/posts/id/mod.rs — the `id` module becomes `{post_id}` in the URL.
/// use topcoat::{
///     context::Cx,
///     router::{RedirectExt, Result, page, path_param},
///     view::view,
/// };
///
/// #[path_param]
/// struct PostId(uuid::Uuid);
///
/// #[page]
/// async fn post_page(cx: &Cx) -> Result {
///     let post_id = PostId::of(cx).as_ref().ok_or_redirect("/invalid-id")?;
///     view! { "showing post with id: " (post_id.to_string()) }
/// }
/// ```
///
/// ## Regular router
///
/// ```ignore
/// // The placeholder `{post_id}` matches the snake-cased struct name `PostId`.
/// #[path_param]
/// struct PostId(uuid::Uuid);
///
/// #[page("/posts/{post_id}")]
/// async fn post_page(cx: &Cx) -> Result {
///     let post_id = PostId::of(cx).as_ref().ok_or_redirect("/invalid-id")?;
///     view! { "showing post with id: " (post_id.to_string()) }
/// }
/// ```
///
/// ## Borrowed `&str` inner type
///
/// ```ignore
/// // No parsing — the raw segment value is exposed directly.
/// #[path_param]
/// struct Slug<'a>(&'a str);
///
/// #[page]
/// async fn show(cx: &Cx) -> Result {
///     let slug = Slug::of(cx); // `&Slug<'_>`
///     view! { "slug: " (&**slug) }
/// }
/// ```
///
/// # Requirements
///
/// - The item must be a tuple struct with exactly one unnamed field.
/// - For non-`&str` inner types, the inner type must implement
///   [`FromStr`](core::str::FromStr) and meet the requirements of
///   [`#[memoize]`](macro@memoize) (the parsed `Result` must be
///   `Send + Sync + 'static`).
#[cfg(feature = "router")]
#[proc_macro_attribute]
pub fn path_param(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = syn::parse_macro_input!(attr as path_param::PathParamAttr);
    let item = syn::parse_macro_input!(item as path_param::PathParamItem);
    let combined = path_param::PathParam::new(attr, item);
    quote! { #combined }.into()
}

/// Declares a typed view of the request's query string.
///
/// Apply this attribute to a struct with named fields. The macro derives
/// [`serde::Deserialize`] on the struct and generates an `of(cx: &Cx)`
/// associated function that parses the query string of whichever request
/// `cx` belongs to, using [`serde_urlencoded`].
///
/// The same struct can be used from any handler — it is not tied to a
/// particular route. `of` returns `&Result<Self, serde_urlencoded::de::Error>`,
/// and parsing is memoized per request so repeated calls within one handler
/// share the same parse result.
///
/// # Examples
///
/// ```ignore
/// use topcoat::{
///     context::Cx,
///     router::{Result, page, query_params},
///     view::view,
/// };
///
/// #[query_params]
/// struct PageQuery {
///     page: Option<u32>,
/// }
///
/// #[page]
/// async fn posts(cx: &Cx) -> Result {
///     // For `/posts?page=2`, this yields `Some(2)`.
///     let q = PageQuery::of(cx).as_ref().unwrap();
///     view! {
///         <div>
///             "currently on page: " (q.page)
///         </div>
///     }
/// }
/// ```
///
/// # Requirements
///
/// - The struct's fields must be deserializable by `serde_urlencoded` (use
///   `Option<T>` for optional parameters, since `serde_urlencoded` does not
///   apply `#[serde(default)]` automatically).
/// - The struct must be `Send + Sync + 'static` to be memoized across the
///   request.
#[cfg(feature = "router")]
#[proc_macro_attribute]
pub fn query_params(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = syn::parse_macro_input!(attr as query_params::QueryParamsAttr);
    let item = syn::parse_macro_input!(item as query_params::QueryParamsItem);
    let combined = query_params::QueryParams::new(attr, item);
    quote! { #combined }.into()
}

/// Caches the result of a function for the duration of a request, keyed by its arguments.
///
/// The annotated function must take a `cx: &Cx` parameter as its handle into the request
/// context. All other arguments form the cache key: the first call with a given set of
/// arguments runs the body and stores the result; subsequent calls with equal arguments
/// return the cached value without re-running the body.
///
/// The function's return type `T` is rewritten to `&T` that has the same lifetime as `&cx`.
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
    let attr = syn::parse_macro_input!(attr as memoize::MemoizeAttr);
    let item = syn::parse_macro_input!(item as memoize::MemoizeItem);
    let memoize = memoize::Memoize::new(attr, item);
    quote! { #memoize }.into()
}
