use proc_macro::TokenStream;
use quote::quote;

/// Declares a page handler.
///
/// The page's URL is the path string given to the attribute (`#[page("/about")]`). When no path is
/// given, the URL is derived from the function's enclosing module path, kebab-cased — provided the
/// function is reachable from a [`module_router!`](../router/macro.module_router.html). Both
/// forms register into the same router, so explicit and module-derived paths can be mixed freely
/// in one app.
///
/// Path strings are Topcoat [`Path`](../router/struct.Path.html)s: literal segments (`users`),
/// `{name}` for dynamic parameters, `{*name}` for wildcard tails, and `(name)` for groups (which
/// participate in layout matching but are stripped from the served URL).
///
/// # Handler signature
///
/// The function is `async` and returns [`Result`](../type.Result.html). It may take
/// [`cx: &Cx`](../context/struct.Cx.html), one request body parameter implementing
/// [`FromRequest`](../router/trait.FromRequest.html), both, or neither. The body parameter may use
/// a destructuring pattern such as `Json(input): Json<T>`. Order does not matter, and there can
/// only be one request body parameter.
///
/// # Examples
///
/// Explicit path:
///
/// ```ignore
/// #[page("/users/{id}")]
/// async fn user_profile() -> Result {
///     view! { <h1>"User profile"</h1> }
/// }
/// ```
///
/// Module-derived path (in `src/app/about.rs` under `module_router!()`, this serves `/about`):
///
/// ```ignore
/// #[page]
/// async fn about() -> Result {
///     view! { <h1>"About"</h1> }
/// }
/// ```
///
/// Reading a request body:
///
/// ```ignore
/// #[page("/contact")]
/// async fn contact(Form(input): Form<Search>) -> Result {
///     view! { <main>"searching for " (input.q)</main> }
/// }
/// ```
#[proc_macro_attribute]
pub fn page(attr: TokenStream, item: TokenStream) -> TokenStream {
    match topcoat_router::ast::page::Page::parse(attr.into(), item.into()) {
        Ok(value) => quote! { #value }.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// Declares a layout that wraps inner pages.
///
/// A layout wraps every page whose URL begins with the layout's URL. The layout's URL is the path
/// string given to the attribute (`#[layout("/settings")]`). When no path is given, it is derived
/// from the function's enclosing module path, kebab-cased — provided the function is reachable
/// from a [`module_router!`](../router/macro.module_router.html). When several layouts match a
/// given page, they nest from least specific (outermost) to most specific (innermost).
///
/// # Handler signature
///
/// The function is `async` and returns [`Result`](../type.Result.html). One parameter must be a
/// [`Slot<'_>`](../router/struct.Slot.html) — a future that resolves to the inner page's rendered
/// output, expected to be `.await`ed somewhere in the layout's view. The function may also take
/// [`cx: &Cx`](../context/struct.Cx.html) and one request body parameter implementing
/// [`FromRequest`](../router/trait.FromRequest.html).
///
/// # Examples
///
/// ```ignore
/// use topcoat::{Result, router::{Slot, layout}, view::view};
///
/// #[layout("/")]
/// async fn root_layout(slot: Slot<'_>) -> Result {
///     view! {
///         <!DOCTYPE html>
///         <html>
///             <body>
///                 <nav><a href="/">"Home"</a></nav>
///                 (slot.await?)
///             </body>
///         </html>
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn layout(attr: TokenStream, item: TokenStream) -> TokenStream {
    match topcoat_router::ast::layout::Layout::parse(attr.into(), item.into()) {
        Ok(value) => quote! { #value }.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// Declares an API route handler.
///
/// A route always declares an HTTP method as its first argument (`GET`, `POST`, `PUT`, `DELETE`,
/// `PATCH`, `HEAD`, or `OPTIONS`). The URL follows the method as an optional path string
/// (`#[route(GET "/api/health")]`); when omitted, it is derived from the function's enclosing
/// module path, kebab-cased — provided the function is reachable from a
/// [`module_router!`](../router/macro.module_router.html). Both forms register into the same
/// router and can be mixed.
///
/// # Handler signature
///
/// The function is `async` and returns `Result<T>` where `T` implements
/// [`IntoResponse`](../router/trait.IntoResponse.html). It may take
/// [`cx: &Cx`](../context/struct.Cx.html), one request body parameter implementing
/// [`FromRequest`](../router/trait.FromRequest.html), both, or neither.
///
/// # Response conversion
///
/// The macro converts the success value via
/// [`IntoResponse::into_response`](../router/trait.IntoResponse.html#tymethod.into_response).
/// Strings, status codes, byte buffers, `(headers, body)` tuples, and
/// [`Json<T>`](../router/struct.Json.html) all work. A raw `Result<T>` is not automatically
/// serialized as JSON — wrap it in `Json<T>` to opt in.
///
/// # Examples
///
/// ```ignore
/// use serde::{Deserialize, Serialize};
/// use topcoat::{Result, router::{Json, route}};
///
/// #[derive(Deserialize, Serialize)]
/// struct CreateUser { name: String }
///
/// #[route(POST "/api/users")]
/// async fn create_user(Json(input): Json<CreateUser>) -> Result<Json<CreateUser>> {
///     Ok(Json(CreateUser { name: input.name }))
/// }
/// ```
#[proc_macro_attribute]
pub fn route(attr: TokenStream, item: TokenStream) -> TokenStream {
    match topcoat_router::ast::route::Route::parse(attr.into(), item.into()) {
        Ok(value) => quote! { #value }.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// Customizes how a module contributes to module-router URLs.
///
/// `segment!(...)` is placed at the top of a module to override the default kebab-cased module
/// name when its descendants build URLs through a
/// [`module_router!`](../router/macro.module_router.html). It has no effect on a regular
/// [`Router`](../router/struct.Router.html), nor on items whose attribute carries an explicit
/// path.
///
/// # Forms
///
/// - `segment!(rename = "name")` — replaces the URL segment with the literal, used as-is (no
///   kebab-casing).
/// - `segment!(kind = Group)` — turns the module into a *group*: it contributes no URL segment but
///   can still hold a shared layout. Equivalent to prefixing the module name with `_`.
/// - `segment!(kind = Static)` — forces a `_`-prefixed module back into a regular static segment.
/// - `segment!(kind = Param, rename = "name")` — turns the module into a dynamic `{name}`
///   parameter. Emitted automatically by [`#[path_param]`](macro@path_param); rarely written by
///   hand.
///
/// # Examples
///
/// ```ignore
/// // src/app/blog_post.rs — module URL becomes `/articles` instead of `/blog-post`.
/// topcoat::router::segment!(rename = "articles");
/// ```
///
/// ```ignore
/// // src/app/marketing/mod.rs — `marketing` contributes no URL segment.
/// topcoat::router::segment!(kind = Group);
/// ```
///
/// ```ignore
/// // src/app/_group/mod.rs — `_group` is reachable as `/group`.
/// topcoat::router::segment!(kind = Static);
/// ```
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
///   emits a [`segment!`](macro@segment)`(kind = Param, rename = "...")` for the enclosing module.
///   The module's URL segment is replaced by the parameter, so a `PostId` defined anywhere in
///   module `app::posts::id` turns that module into `{post_id}` in the URL.
///
/// - **Regular [`Router`](../router/struct.Router.html)** — the page's path string is the source of
///   truth. Include a matching parameter name in the [`#[page("...")]`](macro@page) path; the
///   snake-cased struct name must equal the `{...}` placeholder for `of` to find the value. The
///   [`segment!`](macro@segment) emitted by the macro is inert for this router.
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

#[cfg(feature = "runtime")]
#[proc_macro_attribute]
pub fn action(attr: TokenStream, item: TokenStream) -> TokenStream {
    match topcoat_router::ast::action::Action::parse(attr.into(), item.into()) {
        Ok(value) => quote! { #value }.into(),
        Err(error) => error.to_compile_error().into(),
    }
}
