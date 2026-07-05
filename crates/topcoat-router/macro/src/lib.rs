use proc_macro::TokenStream;
use quote::quote;

/// Declares a page handler.
///
/// The page's URL is the path string given to the attribute (`#[page("/about")]`). When no path is
/// given, the URL is derived from the function's enclosing module path, kebab-cased, provided the
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
/// ```rust
/// # use topcoat::{Result, router::page, view::view};
/// #[page("/users/{id}")]
/// async fn user_profile() -> Result {
///     view! { <h1>"User profile"</h1> }
/// }
/// ```
///
/// Module-derived path (in `src/app/about.rs` under `module_router!()`, this serves `/about`):
///
/// ```rust
/// # use topcoat::{Result, router::page, view::view};
/// #[page]
/// async fn about() -> Result {
///     view! { <h1>"About"</h1> }
/// }
/// ```
///
/// Reading a request body:
///
/// ```rust
/// # use topcoat::{Result, router::{Form, page}, view::view};
/// # use serde::Deserialize;
/// # #[derive(Deserialize)]
/// # struct Search { q: String }
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
/// from the function's enclosing module path, kebab-cased, provided the function is reachable
/// from a [`module_router!`](../router/macro.module_router.html). When several layouts match a
/// given page, they nest from least specific (outermost) to most specific (innermost).
///
/// # Handler signature
///
/// The function is `async` and returns [`Result`](../type.Result.html). One parameter must be a
/// [`Slot<'_>`](../router/struct.Slot.html): a future that resolves to the inner page's rendered
/// output, expected to be `.await`ed somewhere in the layout's view. The function may also take
/// [`cx: &Cx`](../context/struct.Cx.html) and one request body parameter implementing
/// [`FromRequest`](../router/trait.FromRequest.html).
///
/// # Examples
///
/// ```rust
/// use topcoat::{
///     Result,
///     router::{Slot, layout},
///     view::view,
/// };
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
/// module path, kebab-cased, provided the function is reachable from a
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
/// serialized as JSON; wrap it in `Json<T>` to opt in.
///
/// # Examples
///
/// ```rust
/// use serde::{Deserialize, Serialize};
/// use topcoat::{
///     Result,
///     router::{Json, route},
/// };
///
/// #[derive(Deserialize, Serialize)]
/// struct CreateUser {
///     name: String,
/// }
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

/// Declares a layer that wraps the routes nested under its path.
///
/// A layer wraps every matched route whose URL begins with the layer's URL (the same prefix rule as
/// [`#[layout]`](macro@layout)), so a layer at `/admin` wraps only routes under `/admin`, while a
/// layer at `/` wraps everything. The layer's URL is the path string given to the attribute
/// (`#[layer("/admin")]`); when omitted, it is derived from the function's enclosing module path,
/// kebab-cased, provided the function is reachable from a
/// [`module_router!`](../router/macro.module_router.html). When several layers match a route, they
/// nest from least specific (outermost) to most specific (innermost).
///
/// # Handler signature
///
/// The function is `async` and takes [`cx: &mut Cx`](../context/struct.Cx.html), the request
/// [`body: Body`](../router/struct.Body.html), and a [`next:
/// Next<'_>`](../router/struct.Next.html), returning `Result<T>` where `T` implements
/// [`IntoResponse`](../router/trait.IntoResponse.html). Call [`next.run(cx,
/// body)`](../router/struct.Next.html#method.run) to invoke the inner layers and ultimately the
/// route.
///
/// # Examples
///
/// ```rust
/// use topcoat::{
///     Result,
///     context::Cx,
///     router::{Body, Next, Response, layer},
/// };
///
/// #[layer("/")]
/// async fn timing(cx: &mut Cx, body: Body, next: Next<'_>) -> Result<Response> {
///     let start = std::time::Instant::now();
///     let response = next.run(cx, body).await?;
///     println!("handled in {:?}", start.elapsed());
///     Ok(response)
/// }
/// ```
#[proc_macro_attribute]
pub fn layer(attr: TokenStream, item: TokenStream) -> TokenStream {
    match topcoat_router::ast::layer::Layer::parse(attr.into(), item.into()) {
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
/// - `segment!(rename = "name")`: replaces the URL segment with the literal, used as-is (no
///   kebab-casing).
/// - `segment!(kind = Group)`: turns the module into a *group*: it contributes no URL segment but
///   can still hold a shared layout. Equivalent to prefixing the module name with `_`.
/// - `segment!(kind = Static)`: forces a `_`-prefixed module back into a regular static segment.
/// - `segment!(kind = Param, rename = "name")`: turns the module into a dynamic `{name}` parameter.
///   Emitted automatically by [`#[path_param]`](macro@path_param); rarely written by hand.
///
/// # Examples
///
/// ```rust
/// // src/app/blog_post.rs: module URL becomes `/articles` instead of `/blog-post`.
/// topcoat::router::segment!(rename = "articles");
/// ```
///
/// ```rust
/// // src/app/marketing.rs: `marketing` contributes no URL segment.
/// topcoat::router::segment!(kind = Group);
/// ```
///
/// ```rust
/// // src/app/_group.rs: `_group` is reachable as `/group`.
/// topcoat::router::segment!(kind = Static);
/// ```
#[proc_macro]
pub fn segment(tokens: TokenStream) -> TokenStream {
    let segment = syn::parse_macro_input!(tokens as topcoat_router::ast::segment::Segment);
    quote! { #segment }.into()
}

#[doc = include_str!("../docs/path_param.md")]
#[proc_macro_attribute]
pub fn path_param(attr: TokenStream, item: TokenStream) -> TokenStream {
    match topcoat_router::ast::path_param::PathParam::parse(attr.into(), item.into()) {
        Ok(value) => quote! { #value }.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

#[doc = include_str!("../docs/query_params.md")]
#[proc_macro_attribute]
pub fn query_params(attr: TokenStream, item: TokenStream) -> TokenStream {
    match topcoat_router::ast::query_params::QueryParams::parse(attr.into(), item.into()) {
        Ok(value) => quote! { #value }.into(),
        Err(error) => error.to_compile_error().into(),
    }
}
