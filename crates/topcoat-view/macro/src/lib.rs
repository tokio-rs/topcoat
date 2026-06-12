use proc_macro::TokenStream;
use quote::quote;

/// The [`view!`] macro is Topcoat's HTML templating syntax. It tries to be unsurprising by staying
/// close to real HTML instead of inventing a Rust-shaped HTML dialect.
///
/// That means:
///
/// - HTML elements use their real names.
/// - HTML void elements, such as `<br>`, `<hr>`, `<img>`, `<input>`, `<meta>`, and `<link>`, are
///   written without closing tags.
/// - Non-void elements need matching closing tags.
/// - Attribute names can use HTML separators like `-`, `:`, and `.`: `data-post-id`, `aria-label`,
///   `xmlns:xlink`, `hx-get`, `class.active`.
/// - Rust keywords are still valid HTML attribute names, so `type="button"` and `for="email"` work
///   as expected.
/// - Literal text and literal attribute values are string literals.
///
/// ```rust,ignore
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
///             <hr>
///         </body>
///     </html>
/// }
/// ```
///
/// Element names can use dashes, so custom elements fit naturally:
///
/// ```rust,ignore
/// view! {
///     <my-widget data-widget-id="profile"></my-widget>
/// }
/// ```
///
/// ## Rust Expressions
///
/// Use parentheses to interpolate a Rust expression into markup.
///
/// In child position, the expression becomes a node:
///
/// ```rust,ignore
/// view! {
///     <h1>"Hello, " (user.name) "!"</h1>
///     (sidebar)
/// }
/// ```
///
/// In attribute value position, the expression becomes the value:
///
/// ```rust,ignore
/// view! {
///     <a href=(url) aria-current=(is_current)>"Open"</a>
/// }
/// ```
///
/// The same parenthesized expression syntax can also be used for dynamic attribute names and
/// dynamic element names:
///
/// ```rust,ignore
/// let tag = "section";
/// let attr = "data-state";
///
/// view! {
///     <(tag) (attr)="ready">"Loaded"</(tag)>
/// }
/// ```
///
/// Literal text must be quoted because unquoted Rust identifiers are meaningful to the macro:
///
/// ```rust,ignore
/// view! {
///     <p>"This is text"</p>
///     <p>(computed_text)</p>
/// }
/// ```
///
/// ## Control Flow
///
/// Control flow in [`view!`] is Rust control flow with markup bodies. The macro lowers these
/// constructs into ordinary Rust statements that append to the view being built.
///
/// ### `if`
///
/// Use `if`, `else if`, and `else` to choose which markup is emitted.
///
/// ```rust,ignore
/// view! {
///     if user.is_some() {
///         <a href="/account">"Account"</a>
///     } else {
///         <a href="/login">"Sign in"</a>
///     }
/// }
/// ```
///
/// In attributes, each branch emits attributes instead of child nodes:
///
/// ```rust,ignore
/// view! {
///     <a
///         href="/posts"
///         if current {
///             aria-current="page"
///             class="active"
///         }
///     >
///         "Posts"
///     </a>
/// }
/// ```
///
/// ### `for`
///
/// Use `for pat in expr { ... }` to render the body once for each item.
///
/// ```rust,ignore
/// view! {
///     <ul>
///         for post in posts {
///             <li>
///                 <a href=(post.url)>(post.title)</a>
///             </li>
///         }
///     </ul>
/// }
/// ```
///
/// In attributes, a loop can emit zero or more attributes. This is useful when you already have
/// attributes represented as data:
///
/// ```rust,ignore
/// view! {
///     <div
///         for (name, value) in attrs {
///             (name)=(value)
///         }
///     ></div>
/// }
/// ```
///
/// ### `match`
///
/// Use `match` to choose markup from patterns. Match arms can also use guards.
///
/// ```rust,ignore
/// view! {
///     match status {
///         Status::Draft => <span>"Draft"</span>,
///         Status::Published { title } => <a href="/posts">(title)</a>,
///         Status::Archived if show_archived => <span>"Archived"</span>,
///         _ => "",
///     }
/// }
/// ```
///
/// A match arm body is one view node. If a branch needs multiple sibling nodes, wrap them in a
/// block:
///
/// ```rust,ignore
/// view! {
///     match user {
///         Some(user) => {
///             <h1>(user.name)</h1>
///             <p>"Signed in"</p>
///         },
///         None => <a href="/login">"Sign in"</a>,
///     }
/// }
/// ```
///
/// In attributes, each arm emits one attribute node:
///
/// ```rust,ignore
/// view! {
///     <article
///         match state {
///             State::Open => class="open",
///             State::Closed => aria-disabled=(true),
///         }
///     ></article>
/// }
/// ```
///
/// For multiple conditional attributes, put the `if`, `for`, or `match` at the level where it can
/// emit the attributes you need.
///
/// ### `let`
///
/// Use `let pat = expr;` to bind values for later nodes in the same body.
///
/// ```rust,ignore
/// view! {
///     <article>
///         let title = post.title.trim();
///
///         <h1>(title)</h1>
///         <a href=(post.url)>"Read"</a>
///     </article>
/// }
/// ```
///
/// The same works in an attribute list. The binding is in scope for attributes that follow it:
///
/// ```rust,ignore
/// view! {
///     <a
///         let href = post.url();
///         href=(href)
///         data-slug=(post.slug)
///     >
///         (post.title)
///     </a>
/// }
/// ```
///
/// ## Components
///
/// Components are called inside [`view!`] with function-call syntax. Named arguments use `name:
/// value`, and child nodes can be passed after the named arguments:
///
/// ```rust,ignore
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
/// All component parameters are named parameters, except `child`, which can be passed unnamed in
/// the last position. Conceptually, those trailing child nodes are the same thing as a `child`
/// parameter whose value is a [`view! { ... }`][`view!`] containing those nodes.
///
/// See the [`component`] macro guide in [component.md](component.md) for defining components and
/// passing child content.
///
/// ## Conditional Attributes
///
/// Expression attributes can remove themselves from the rendered markup.
///
/// When an attribute value evaluates to [`false`], the whole attribute is omitted. When it
/// evaluates to [`None`], the whole attribute is omitted. [`Some(value)`][`Some`] renders the
/// attribute using the inner value.
///
/// ```rust,ignore
/// view! {
///     <button
///         disabled=(is_disabled)
///         aria-current=(is_current.then_some("page"))
///         title=(maybe_title)
///     >
///         "Save"
///     </button>
/// }
/// ```
///
/// If the values are:
///
/// ```rust,ignore
/// let is_disabled = false;
/// let is_current = true;
/// let maybe_title: Option<&str> = None;
/// ```
///
/// then the rendered opening tag includes `aria-current="page"`, but leaves out `disabled` and
/// `title` completely.
///
/// This omission logic applies to expression attributes. Literal attributes are always present:
///
/// ```rust,ignore
/// view! {
///     <button disabled="false">"Still disabled in HTML"</button>
/// }
/// ```
///
/// For reusable runtime attribute collections, use the [`attributes!`] macro. The [attributes
/// guide](attributes.md) covers the same attribute syntax and the [`topcoat::view::Attributes`]
/// value that can be passed around and inserted into an element as an attribute fragment.
///
/// ## Custom Values In Markup
///
/// The macro accepts dynamic Rust values by routing them through small runtime traits. Implement
/// the trait for the position where your type should be accepted:
///
/// - [`NodeViewParts`] for values used as child nodes: `(value)`.
/// - [`AttributeValueViewParts`] for values used as attribute values: `name=(value)`.
/// - [`AttributeKeyViewParts`] for values used as dynamic attribute names: `(name)="value"`.
/// - [`AttributeViewParts`] for values that emit one or more full attributes in APIs that accept
///   complete attribute fragments.
/// - [`ElementNameViewParts`] for values used as dynamic element names: `<(name)>...</(name)>`.
///
/// For example, a type can opt into child-node rendering by implementing [`NodeViewParts`]:
///
/// ```rust,ignore
/// use topcoat::view::{NodeViewParts, ViewParts};
///
/// struct Badge(String);
///
/// impl NodeViewParts for Badge {
///     fn into_view_parts(self, parts: &mut ViewParts) {
///         parts.push(self.0);
///     }
/// }
///
/// view! {
///     <p>(Badge("New".to_owned()))</p>
/// }
/// ```
///
/// For attribute values, implement [`AttributeValueViewParts`]. Its
/// [`attribute_present`][AttributeValueViewParts::attribute_present] method controls whether the
/// containing attribute is rendered at all.
///
/// ```rust,ignore
/// use topcoat::view::{AttributeValueViewParts, ViewParts};
///
/// struct DataId(Option<String>);
///
/// impl AttributeValueViewParts for DataId {
///     fn attribute_present(&self) -> bool {
///         self.0.is_some()
///     }
///
///     fn into_view_parts(self, parts: &mut ViewParts) {
///         if let Some(value) = self.0 {
///             parts.push(value);
///         }
///     }
/// }
///
/// view! {
///     <article data-id=(DataId(Some("post-1".to_owned())))></article>
/// }
/// ```
///
/// [`AttributeKeyViewParts`]: trait.AttributeKeyViewParts.html
/// [`AttributeValueViewParts`]: trait.AttributeValueViewParts.html
/// [AttributeValueViewParts::attribute_present]: trait.AttributeValueViewParts.html#tymethod.attribute_present
/// [`AttributeViewParts`]: trait.AttributeViewParts.html
/// [`ElementNameViewParts`]: trait.ElementNameViewParts.html
/// [`NodeViewParts`]: trait.NodeViewParts.html
/// [`component`]: attr.component.html
/// [`attributes!`]: macro.attributes.html
/// [`false`]: https://doc.rust-lang.org/std/keyword.false.html
/// [`None`]: https://doc.rust-lang.org/std/option/enum.Option.html#variant.None
/// [`Some`]: https://doc.rust-lang.org/std/option/enum.Option.html#variant.Some
/// [`topcoat::view::Attributes`]: struct.Attributes.html
/// [`view!`]: macro.view.html
#[proc_macro]
pub fn view(tokens: TokenStream) -> TokenStream {
    let parsed = syn::parse_macro_input!(tokens as topcoat_view::ast::view::View);
    quote! { #parsed }.into()
}

/// The [`attributes!`] macro builds a [`topcoat::view::Attributes`] value from Topcoat's attribute
/// syntax.
///
/// Use it when attributes need to be passed around, assembled outside a [`view!`] call, changed at
/// runtime, or forwarded through components.
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
/// ## Syntax
///
/// The body of [`attributes!`] has the same syntax as attributes inside an element in [`view!`].
///
/// That includes literal attributes, expression values, dynamic names, binding attributes, event
/// handlers, and attribute-level control flow:
///
/// ```rust,ignore
/// use topcoat::view::attributes;
///
/// let id = "submit";
/// let extra = [("data-state", "ready"), ("data-size", "compact")];
///
/// let attrs = attributes! {
///     class="button"
///     id=(id)
///     :data-bound=$(id.to_owned())
///     @input="(e) => console.log(e)"
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
///
///     match id {
///         "submit" => aria-label="Submit",
///         _ => aria-label="Button",
///     }
/// };
/// ```
///
/// [`attributes!`] produces attributes, not child nodes. Control-flow bodies inside the macro
/// therefore emit attributes in the same way they do inside a [`view!`] element's opening tag.
///
/// ## Runtime Attributes
///
/// The generated value is [`topcoat::view::Attributes`]. It is a runtime collection of attributes
/// with unique keys.
///
/// ```rust,ignore
/// use topcoat::view::attributes;
///
/// let mut attrs = attributes! {
///     class="button"
///     data-state="idle"
/// };
///
/// attrs.insert("data-state", "loading");
/// attrs.insert("disabled", true);
///
/// assert!(attrs.contains_key("class"));
/// ```
///
/// Because [`Attributes`] is map-like, each key appears at most once. Inserting the same key again
/// replaces the previous value. Do not rely on render order for attributes.
///
/// ## Inserting Attributes Into Elements
///
/// Insert an [`Attributes`] value into an element by using it as a parenthesized attribute
/// fragment:
///
/// ```rust,ignore
/// use topcoat::view::{attributes, view};
///
/// let attrs = attributes! {
///     class="card"
///     data-kind="summary"
/// };
///
/// view! {
///     <article (attrs)>
///         <h2>"Summary"</h2>
///     </article>
/// }
/// ```
///
/// Any type that implements [`AttributeViewParts`] can be used in the same position. [`Attributes`]
/// implements that trait, so it works as a complete reusable attribute fragment.
///
/// Inserting an [`Attributes`] value consumes it. Clone the value first if the same attribute
/// collection needs to be inserted into more than one element.
///
/// ## Passing Attributes To Components
///
/// Components can accept [`Attributes`] as a normal argument. This is useful for forwarding
/// caller-controlled attributes to the component's root element.
///
/// ```rust,ignore
/// use topcoat::{
///     Result,
///     view::{Attributes, View, attributes, component, view},
/// };
///
/// #[component]
/// async fn panel(attrs: Attributes, child: View) -> Result {
///     view! {
///         <section (attrs)>
///             (child)
///         </section>
///     }
/// }
///
/// view! {
///     panel(
///         attrs: attributes! {
///             class="panel"
///             data-panel="account"
///         },
///         <p>"Account settings"</p>
///     )
/// }
/// ```
///
/// Since the value is ordinary Rust data, you can build it in helper functions, add or replace
/// attributes before rendering, and pass it through several layers before inserting it into an
/// element.
///
/// [`AttributeViewParts`]: trait.AttributeViewParts.html
/// [`Attributes`]: struct.Attributes.html
/// [`attributes!`]: macro.attributes.html
/// [`topcoat::view::Attributes`]: struct.Attributes.html
/// [`view!`]: macro.view.html
#[proc_macro]
pub fn attributes(tokens: TokenStream) -> TokenStream {
    let parsed = syn::parse_macro_input!(tokens as topcoat_view::ast::attributes::Attributes);
    quote! { #parsed }.into()
}

/// Components are async functions annotated with [`#[component]`][`component`]. They return a
/// [`View`] through the usual Topcoat [`Result`] type, and can take typed parameters like any other
/// Rust function.
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
/// ```
///
/// ## Calling Components
///
/// Call components inside [`view!`] with function-call syntax. Named arguments use `name: value`:
///
/// ```rust,ignore
/// view! {
///     <header>
///         badge(
///             label: "New",
///             tone: "success",
///         )
///     </header>
/// }
/// ```
///
/// All component parameters are named parameters, except `child`, which can be passed unnamed in
/// the last position. After the named arguments, unnamed child nodes are written like normal
/// [`view!`] content; multiple child nodes do not need commas between them.
///
/// ## Child Content
///
/// If a component accepts a parameter named `child` with type [`View`], any extra view nodes in the
/// call are collected and passed as that child view.
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
/// Conceptually, those trailing child nodes are the same thing as a `child` parameter whose value
/// is a [`view! { ... }`][`view!`] containing those nodes.
///
/// ## Props
///
/// The macro turns the function's parameters (except `cx`) into a generated props struct named
/// after the component in PascalCase plus `Props` (`badge` becomes `BadgeProps`), which derives
/// [`Props`] to get a typestate builder. Component calls in [`view!`] go through that builder, so
/// leaving out a parameter is a compile error naming the missing property.
///
/// Parameters can use the same attributes as [`Props`] fields:
///
/// - `#[default]` makes the parameter optional; when not passed, it gets `Default::default()`.
/// - `#[into]` lets callers pass anything that converts via `Into`.
///
/// ```rust,ignore
/// #[component]
/// async fn badge(#[into] label: String, #[default] tone: Tone) -> Result {
///     // ...
/// }
/// ```
///
/// ## Generics
///
/// Components can be generic; the function's generics carry over to the props struct. Because
/// component futures must be `Send`, type parameters stored in props need a `Send` bound (and
/// `Sync` when the view borrows them):
///
/// ```rust,ignore
/// #[component]
/// async fn count<T: Send + Sync>(items: Vec<T>) -> Result {
///     view! { <span>(items.len())</span> }
/// }
/// ```
///
/// `impl Trait` parameters work too. Each occurrence is lifted into a generic type parameter on
/// the props struct, keeping its bounds and adding `Send`:
///
/// ```rust,ignore
/// #[component]
/// async fn shout(label: impl Into<String>) -> Result {
///     view! { <b>(label.into().to_uppercase())</b> }
/// }
/// ```
///
/// ## Request Context
///
/// Components can ask for the current request context by declaring a `cx` parameter that borrows
/// [`Cx`]:
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
///
/// [`Cx`]: ../context/struct.Cx.html
/// [`Props`]: derive.Props.html
/// [`Result`]: ../type.Result.html
/// [`View`]: struct.View.html
/// [`component`]: attr.component.html
/// [`view!`]: macro.view.html
#[proc_macro_attribute]
pub fn component(attr: TokenStream, item: TokenStream) -> TokenStream {
    match topcoat_view::ast::component::Component::parse(attr.into(), item.into()) {
        Ok(value) => quote! { #value }.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// Derives a typestate builder for a props struct.
///
/// For a struct `ButtonProps`, the derive generates a `ButtonPropsBuilder` whose `build()` method
/// only becomes available once every required property has been set. Forgetting a property is a
/// compile error, not a runtime panic.
///
/// ```rust,ignore
/// use topcoat::view::Props;
///
/// #[derive(Props)]
/// struct ButtonProps {
///     #[into]
///     label: String,
///     kind: ButtonKind,
///     #[default]
///     disabled: bool,
/// }
///
/// let props = ButtonProps::builder()
///     .label("Save")
///     .kind(ButtonKind::Primary)
///     .build();
/// ```
///
/// ## Field Attributes
///
/// - `#[default]` makes a property optional. If it is not set, the field is filled with
///   [`Default::default()`]. The field's type must implement [`Default`].
/// - `#[into]` makes the generated setter accept `impl Into<T>` instead of `T`, so
///   `.label("Save")` works for a `String` field.
///
/// Both attributes can be combined on the same field.
///
/// ## Typestate
///
/// The builder tracks each required property in a type parameter that flips to [`Set`] when the
/// property's setter is called. `build()` requires every marker to implement [`IsSet`], so this
/// fails to compile:
///
/// ```rust,ignore
/// // error: missing required property `kind`
/// let props = ButtonProps::builder().label("Save").build();
/// ```
///
/// Setters can be called more than once; later calls replace the earlier value.
///
/// ## Generics
///
/// Generic structs are supported. The struct's generics, bounds, and `where` clauses carry over
/// to the builder:
///
/// ```rust,ignore
/// #[derive(Props)]
/// struct ListProps<T: Clone> {
///     items: Vec<T>,
///     #[default]
///     compact: bool,
/// }
/// ```
///
/// ## The `Props` Trait
///
/// The derive also implements the [`Props`] trait, whose associated `Builder` type names the
/// builder in its initial state. The generated `builder()` function is available both as an
/// inherent function and through the trait.
///
/// [`Default`]: https://doc.rust-lang.org/std/default/trait.Default.html
/// [`Default::default()`]: https://doc.rust-lang.org/std/default/trait.Default.html#tymethod.default
/// [`IsSet`]: trait.IsSet.html
/// [`Props`]: trait.Props.html
/// [`Set`]: struct.Set.html
#[proc_macro_derive(Props, attributes(default, into))]
pub fn props(item: TokenStream) -> TokenStream {
    match topcoat_view::ast::props::Props::parse(item.into()) {
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
