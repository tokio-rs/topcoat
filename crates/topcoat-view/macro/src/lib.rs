use proc_macro::TokenStream;
use quote::quote;

/// Builds a [`topcoat::view::View`](https://docs.rs/topcoat/latest/topcoat/view/struct.View.html)
/// from Topcoat's HTML-like template syntax.
///
/// `view!` is the primary way to write markup in Topcoat. The syntax stays
/// close to HTML: element names use their normal spelling, attribute names
/// can use HTML separators such as `-`, `:`, and `.`, HTML void elements are
/// written without closing tags, and non-void elements use matching closing
/// tags. Rust keywords are valid attribute names, so attributes such as
/// `type="button"` and `for="email"` work as expected.
///
/// Literal text and literal attribute values are string literals. Parenthesized
/// Rust expressions interpolate dynamic values. In child position, an
/// expression becomes a node. In attribute value position, it becomes the
/// attribute value. The same parenthesized form can also be used for dynamic
/// element names, dynamic attribute names, and complete attribute fragments.
///
/// The macro accepts Rust control flow in view bodies:
///
/// - `if`, `else if`, and `else` choose which nodes are emitted.
/// - `for pat in expr { ... }` emits the body once for each item.
/// - `match` chooses one node per arm; use a block when an arm needs multiple
///   sibling nodes.
/// - `let pat = expr;` binds values for nodes that follow in the same body.
///
/// The same control-flow forms can be used inside an element's attribute list,
/// where their bodies emit attributes instead of child nodes.
///
/// Components are called with function-call syntax. Named arguments use
/// `name: value`; any trailing unnamed view nodes are collected as the
/// component's `child` content.
///
/// Expression attributes can remove themselves from the rendered element.
/// A `false` value omits the whole attribute, `None` omits the whole
/// attribute, and `Some(value)` renders the attribute with the inner value.
/// Literal attributes are always present.
///
/// # Examples
///
/// ```rust,ignore
/// use topcoat::view::view;
///
/// view! {
///     <!DOCTYPE html>
///     <html>
///         <head>
///             <meta charset="utf-8">
///             <link rel="stylesheet" href="/app.css">
///         </head>
///         <body>
///             <label for="email">"Email"</label>
///             <input type="email" id="email" aria-label="Email address">
///         </body>
///     </html>
/// }
/// ```
///
/// ```rust,ignore
/// view! {
///     <ul>
///         for post in posts {
///             <li>
///                 <a href=(post.url) aria-current=(post.current.then_some("page"))>
///                     (post.title)
///                 </a>
///             </li>
///         }
///     </ul>
/// }
/// ```
///
/// ```rust,ignore
/// let tag = "section";
/// let attr = "data-state";
///
/// view! {
///     <(tag) (attr)="ready">
///         "Loaded"
///     </(tag)>
/// }
/// ```
#[proc_macro]
pub fn view(tokens: TokenStream) -> TokenStream {
    let parsed = syn::parse_macro_input!(tokens as topcoat_view::ast::view::View);
    quote! { #parsed }.into()
}

/// Builds a [`topcoat::view::Attributes`](https://docs.rs/topcoat/latest/topcoat/view/struct.Attributes.html)
/// value from Topcoat's attribute syntax.
///
/// Use `attributes!` when attributes need to be assembled outside a `view!`
/// call, passed through components, changed at runtime, or reused as a
/// complete attribute fragment. Insert the resulting value into an element
/// with parenthesized attribute-fragment syntax: `<button (attrs)>`.
///
/// The body of `attributes!` accepts the same syntax as an element's attribute
/// list inside `view!`:
///
/// - literal attributes such as `class="button"`;
/// - expression values such as `id=(id)`;
/// - dynamic attribute names such as `(name)="value"`;
/// - complete attribute fragments such as `(attrs)`;
/// - binding attributes such as `:value=$(expr)`;
/// - event handler attributes such as `@input="..."`;
/// - `if`, `for`, `match`, and `let` at attribute-list position.
///
/// `attributes!` produces attributes, not child nodes. Control-flow bodies
/// therefore emit attributes in the same way they do inside a `view!` opening
/// tag.
///
/// The returned `Attributes` value is map-like: each key appears at most once,
/// and inserting the same key again replaces the previous value. Do not rely
/// on render order.
///
/// # Examples
///
/// ```rust,ignore
/// use topcoat::view::{attributes, view};
///
/// let attrs = attributes! {
///     class="button"
///     type="submit"
///     aria-label="Save changes"
/// };
///
/// view! {
///     <button (attrs)>"Save"</button>
/// }
/// ```
///
/// ```rust,ignore
/// let id = "submit";
/// let extra = [
///     ("data-state", "ready"),
///     ("data-size", "compact"),
/// ];
///
/// let attrs = attributes! {
///     class="button"
///     id=(id)
///
///     if id == "submit" {
///         type="submit"
///     } else {
///         type="button"
///     }
///
///     for (name, value) in extra {
///         (name)=(value)
///     }
/// };
/// ```
#[proc_macro]
pub fn attributes(tokens: TokenStream) -> TokenStream {
    let parsed = syn::parse_macro_input!(tokens as topcoat_view::ast::attributes::Attributes);
    quote! { #parsed }.into()
}

/// Defines a Topcoat component from an async function.
///
/// Components are typed, async view functions that can be called from `view!`
/// with function-call syntax. A component function must be an `async fn`, must
/// have an explicit return type whose resolved output is
/// `Result<View, topcoat::Error>`, and must use identifier patterns for its
/// parameters. The usual spelling is `topcoat::Result`.
///
/// Each ordinary function parameter becomes a named component argument. When
/// calling the component in `view!`, pass those arguments with `name: value`.
///
/// Two parameter names have special meaning:
///
/// - `child: View` receives any trailing unnamed view nodes passed after the
///   named arguments in the component call.
/// - `cx: &Cx` receives the current request context.
///
/// If a component accepts `child`, callers can pass child content directly in
/// the call. Conceptually, those trailing nodes are the same as a `child`
/// argument whose value is a `view! { ... }` containing those nodes.
///
/// # Examples
///
/// ```rust,ignore
/// use topcoat::{
///     Result,
///     view::{component, view},
/// };
///
/// #[component]
/// async fn badge(label: &str, tone: &str) -> Result {
///     view! {
///         <span class=(format!("badge badge-{tone}"))>
///             (label)
///         </span>
///     }
/// }
///
/// view! {
///     badge(
///         label: "New",
///         tone: "success",
///     )
/// }
/// ```
///
/// ```rust,ignore
/// use topcoat::{
///     Result,
///     view::{View, component, view},
/// };
///
/// #[component]
/// async fn panel(title: &str, child: View) -> Result {
///     view! {
///         <section class="panel">
///             <h2>(title)</h2>
///             <div class="panel-body">
///                 (child)
///             </div>
///         </section>
///     }
/// }
///
/// view! {
///     panel(
///         title: "Profile",
///         <p>"Account details"</p>
///         badge(
///             label: "Active",
///             tone: "success",
///         )
///     )
/// }
/// ```
///
/// ```rust,ignore
/// use topcoat::{
///     Result,
///     context::Cx,
///     router::uri,
///     view::{component, view},
/// };
///
/// #[component]
/// async fn current_path(cx: &Cx) -> Result {
///     view! {
///         <span>(uri(cx).path())</span>
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn component(attr: TokenStream, item: TokenStream) -> TokenStream {
    match topcoat_view::ast::component::Component::parse(attr.into(), item.into()) {
        Ok(value) => quote! { #value }.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

#[proc_macro_attribute]
pub fn shard(attr: TokenStream, item: TokenStream) -> TokenStream {
    match topcoat_view::ast::shard::Shard::parse(attr.into(), item.into()) {
        Ok(value) => quote! { #value }.into(),
        Err(error) => error.to_compile_error().into(),
    }
}
