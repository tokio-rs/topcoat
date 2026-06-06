use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_attribute]
pub fn page(attr: TokenStream, item: TokenStream) -> TokenStream {
    match topcoat_router::ast::page::Page::parse(attr.into(), item.into()) {
        Ok(value) => quote! { #value }.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

#[proc_macro_attribute]
pub fn layout(attr: TokenStream, item: TokenStream) -> TokenStream {
    match topcoat_router::ast::layout::Layout::parse(attr.into(), item.into()) {
        Ok(value) => quote! { #value }.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

#[proc_macro_attribute]
pub fn route(attr: TokenStream, item: TokenStream) -> TokenStream {
    match topcoat_router::ast::route::Route::parse(attr.into(), item.into()) {
        Ok(value) => quote! { #value }.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

#[proc_macro]
pub fn segment(tokens: TokenStream) -> TokenStream {
    let segment = syn::parse_macro_input!(tokens as topcoat_router::ast::segment::Segment);
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
/// - **Module router** ([`module_router!`](../router/macro.module_router.html)) — the macro also
///   emits a `segment!(kind = Param, rename = "...")` for the enclosing module. The module's URL
///   segment is replaced by the parameter, so a `PostId` defined anywhere in module
///   `app::posts::id` turns that module into `{post_id}` in the URL.
///
/// - **Regular [`Router`](../router/struct.Router.html)** — the page's path string is the source of
///   truth. Include a matching parameter name in the `#[page("...")]` path; the snake-cased struct
///   name must equal the `{...}` placeholder for `of` to find the value. The `segment!` emitted by
///   the macro is inert for this router.
///
/// # Reading the parameter
///
/// The macro generates an `of(cx: &Cx)` associated function whose return
/// type depends on the inner type:
///
/// - **`&str`** — returns `&Self` directly with the borrowed segment value.
/// - **Any other type** — returns `Result<&Self, &<T as FromStr>::Err>`, parsed via
///   [`FromStr`](core::str::FromStr). Parsing is memoized per request, so repeated calls within a
///   handler do not re-parse.
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
///     Result,
///     router::{RouterErrorExt, page, path_param},
///     view::view,
/// };
///
/// #[path_param]
/// struct PostId(uuid::Uuid);
///
/// #[page]
/// async fn post_page(cx: &Cx) -> Result {
///     let post_id = PostId::of(cx).ok_or_redirect("/invalid-id")?;
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
///     let post_id = PostId::of(cx).ok_or_redirect("/invalid-id")?;
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
/// - For non-`&str` inner types, the inner type must implement [`FromStr`](core::str::FromStr) and
///   meet the requirements of [`#[memoize]`](macro@memoize) (the parsed `Result` must be `Send +
///   Sync + 'static`).
#[proc_macro_attribute]
pub fn path_param(attr: TokenStream, item: TokenStream) -> TokenStream {
    match topcoat_router::ast::path_param::PathParam::parse(attr.into(), item.into()) {
        Ok(value) => quote! { #value }.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// Declares a typed view of the request's query string.
///
/// Apply this attribute to a struct with named fields. The macro derives
/// [`serde::Deserialize`] on the struct and generates an `of(cx: &Cx)`
/// associated function that parses the query string of whichever request
/// `cx` belongs to, using [`serde_urlencoded`].
///
/// The same struct can be used from any handler — it is not tied to a
/// particular route. `of` returns `Result<&Self, &serde_urlencoded::de::Error>`,
/// and parsing is memoized per request so repeated calls within one handler
/// share the same parse result.
///
/// # Examples
///
/// ```ignore
/// use topcoat::{
///     context::Cx,
///     Result,
///     router::{page, query_params},
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
///     let q = PageQuery::of(cx).unwrap();
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
/// - The struct's fields must be deserializable by `serde_urlencoded` (use `Option<T>` for optional
///   parameters, since `serde_urlencoded` does not apply `#[serde(default)]` automatically).
/// - The struct must be `Send + Sync + 'static` to be memoized across the request.
#[proc_macro_attribute]
pub fn query_params(attr: TokenStream, item: TokenStream) -> TokenStream {
    match topcoat_router::ast::query_params::QueryParams::parse(attr.into(), item.into()) {
        Ok(value) => quote! { #value }.into(),
        Err(error) => error.to_compile_error().into(),
    }
}
