The [`view!`] macro is Topcoat's HTML templating syntax. It tries to be unsurprising by staying close to real HTML instead of inventing a Rust-shaped HTML dialect. That means:

- HTML elements use their real names.
- HTML void elements, such as `<br>`, `<hr>`, `<img>`, `<input>`, `<meta>`, and `<link>`, are written without closing tags.
- Non-void elements need matching closing tags.
- Attribute names can use HTML separators like `-`, `:`, and `.`: `data-post-id`, `aria-label`, `xmlns:xlink`, `hx-get`, `class.active`.
- Rust keywords are still valid HTML attribute names, so `type="button"` and `for="email"` work as expected.

Unlike HTML however, text nodes must be quoted.

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn example() -> Result {
view! {
    <!DOCTYPE html>
    <html>
        <head>
            <meta charset="utf-8">
            <link rel="stylesheet" href="/app.css">
        </head>
        <body>
            <label for="email">"Email"</label>
            <input type="email" id="email" aria-label="Email address">
            <hr>
        </body>
    </html>
}
# }
```

Element names can use dashes, so custom elements fit naturally:

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn example() -> Result {
view! {
    <my-widget data-widget-id="profile"></my-widget>
}
# }
```

# Rust Expressions

Use parentheses to interpolate a Rust expression into markup.

In child position, the expression becomes a node:

```rust
# use topcoat::{Result, view::*};
# struct User { name: &'static str }
# #[component]
# async fn example() -> Result {
# let user = User { name: "Ada" };
# let sidebar = view! { <aside></aside> }?;
view! {
    <h1>"Hello, " (user.name) "!"</h1>
    (sidebar)
}
# }
```

In attribute value position, the expression becomes the value:

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn example() -> Result {
# let url = "/posts";
# let is_current = true;
view! {
    <a href=(url) aria-current=(is_current)>"Open"</a>
}
# }
```

The same parenthesized expression syntax can also be used for dynamic attribute names and dynamic element names:

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn example() -> Result {
let tag = "section";
let attr = "data-state";

view! {
    <(tag) (attr)="ready">"Loaded"</(tag)>
}
# }
```

Due to a limitation in Rust macros, text nodes must be quoted:

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn example() -> Result {
# let computed_text = "Computed";
view! {
    <p>"This is text"</p>
    <p>(computed_text)</p>
}
# }
```

# Control Flow

Control flow in [`view!`] is Rust control flow with markup bodies. The macro lowers these constructs into ordinary Rust statements that append to the view being built.

## `if`

Use `if`, `else if`, and `else` to choose which markup is emitted.

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn example() -> Result {
# let user: Option<()> = None;
view! {
    if user.is_some() {
        <a href="/account">"Account"</a>
    } else {
        <a href="/login">"Sign in"</a>
    }
}
# }
```

In attributes, each branch emits attributes instead of child nodes:

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn example() -> Result {
# let current = true;
view! {
    <a
        href="/posts"
        if current {
            aria-current="page"
            class="active"
        }
    >
        "Posts"
    </a>
}
# }
```

## `for`

Use `for pat in expr { ... }` to render the body once for each item.

```rust
# use topcoat::{Result, view::*};
# struct Post { url: &'static str, title: &'static str }
# #[component]
# async fn example() -> Result {
# let posts = vec![Post { url: "/a", title: "A" }];
view! {
    <ul>
        for post in posts {
            <li>
                <a href=(post.url)>(post.title)</a>
            </li>
        }
    </ul>
}
# }
```

In attributes, a loop can emit zero or more attributes. This is useful when you already have attributes represented as data:

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn example() -> Result {
# let attrs = vec![("data-id", "1")];
view! {
    <div
        for (name, value) in attrs {
            (name)=(value)
        }
    ></div>
}
# }
```

## `match`

Use `match` to choose markup from patterns. Match arms can also use guards.

```rust
# use topcoat::{Result, view::*};
# enum Status { Draft, Published { title: &'static str }, Archived }
# #[component]
# async fn example() -> Result {
# let status = Status::Draft;
# let show_archived = true;
view! {
    match status {
        Status::Draft => <span>"Draft"</span>,
        Status::Published { title } => <a href="/posts">(title)</a>,
        Status::Archived if show_archived => <span>"Archived"</span>,
        _ => "",
    }
}
# }
```

A match arm body is one view node. If a branch needs multiple sibling nodes, wrap them in a block:

```rust
# use topcoat::{Result, view::*};
# struct User { name: &'static str }
# #[component]
# async fn example() -> Result {
# let user: Option<User> = None;
view! {
    match user {
        Some(user) => {
            <h1>(user.name)</h1>
            <p>"Signed in"</p>
        },
        None => <a href="/login">"Sign in"</a>,
    }
}
# }
```

In attributes, each arm can emit attribute nodes:

```rust
# use topcoat::{Result, view::*};
# enum State { Open, Closed }
# #[component]
# async fn example() -> Result {
# let state = State::Open;
view! {
    <article
        match state {
            State::Open => class="open",
            State::Closed => aria-disabled=(true),
        }
    ></article>
}
# }
```

## `let`

Use `let pat = expr;` to bind values for later nodes in the same body.

```rust
# use topcoat::{Result, view::*};
# struct Post { title: &'static str, url: &'static str }
# #[component]
# async fn example() -> Result {
# let post = Post { title: " Hello ", url: "/hello" };
view! {
    <article>
        let title = post.title.trim();

        <h1>(title)</h1>
        <a href=(post.url)>"Read"</a>
    </article>
}
# }
```

The same works in an attribute list. The binding is in scope for attributes that follow it:

```rust
# use topcoat::{Result, view::*};
# struct Post { slug: &'static str, title: &'static str }
# impl Post { fn url(&self) -> &str { "/hello" } }
# #[component]
# async fn example() -> Result {
# let post = Post { slug: "hello", title: "Hello" };
view! {
    <a
        let href = post.url();
        href=(href)
        data-slug=(post.slug)
    >
        (post.title)
    </a>
}
# }
```

# Components

Components are called inside [`view!`] with a call syntax similar to functions. The macro introduces named parameters with the comma-separated `name: value` syntax to improve readability for components with many (optional) parameters. If the component has a `child` property, you may pass any number of view nodes at the end of parameter list. These do not need to be comma-separated:

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn panel(title: &str, child: View) -> Result { view! { <section>(title)(child)</section> } }
# #[component]
# async fn badge(label: &str, tone: &str) -> Result { view! { <span>(label)(tone)</span> } }
# #[component]
# async fn example() -> Result {
view! {
    panel(
        // Named title parameter:
        title: "Profile",
        // Child nodes:
        <p>"Account details"</p>
        badge(
            label: "Active",
            tone: "success",
        )
    )
}
# }
```

The child nodes desugar to:

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn panel(title: &str, child: View) -> Result { view! { <section>(title)(child)</section> } }
# #[component]
# async fn badge(label: &str, tone: &str) -> Result { view! { <span>(label)(tone)</span> } }
# #[component]
# async fn example() -> Result {
view! {
    panel(
        title: "Profile",
        // Named child parameter:
        child: view! {
            <p>"Account details"</p>
            badge(
                label: "Active",
                tone: "success",
            )
        }?
    )
}
# }
```

See how to define components in the [`component`] macro guide.

# Conditional Attributes

Expression attributes can remove themselves from the rendered markup.

When an attribute value evaluates to [`false`] or [`None`], the whole attribute is omitted. This matches the required [boolean HTML attributes](https://developer.mozilla.org/en-US/docs/Glossary/Boolean/HTML) behavior.

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn example() -> Result {
let is_disabled = false;
let is_current = true;
let maybe_title: Option<&str> = None;

view! {
    <button
        disabled=(is_disabled)
        aria-current=(is_current.then_some("page"))
        title=(maybe_title)
    >
        "Save"
    </button>
}
# }
```

The rendered opening tag includes `aria-current="page"`, but leaves out `disabled` and `title` completely.

This omission logic applies to expression attributes. Literal attributes are always present:

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn example() -> Result {
view! {
    <button disabled="false">"Still disabled in HTML"</button>
}
# }
```

For reusable runtime attribute collections, use the [`attributes!`] macro. It has the same attribute syntax as the [`view!`] macro but generates an [`topcoat::view::Attributes`] value that can be passed around and inserted into an element as an attribute fragment.

To assemble a `class` attribute value from static and conditional parts, use the [`class!`] macro. It builds a [`topcoat::view::Class`] value whose entries join with single spaces, and the attribute is omitted entirely when no entry is present.

# Rendering Outside A Component

Inside a [`component`], `#[page]`, or `#[layout]`, the request context is in scope implicitly, so `view!` can render components and reactive markup with no ceremony. In a plain function you need to pass it at the start of the `view!` macro explicitely:

```rust
# use topcoat::{Result, context::Cx, view::*};
# #[component]
# async fn greeting(name: &str) -> Result { view! { <h1>(name)</h1> } }
async fn render(cx: &Cx) -> Result {
    view! { cx => greeting(name: "World") }
}
```

# Custom Values In Markup

The macro accepts dynamic Rust values by routing them through small runtime traits. Implement the trait for the position where your type should be accepted:

- [`NodeViewParts`] for values used as child nodes: `(value)`.
- [`AttributeValueViewParts`] for values used as attribute values: `name=(value)`.
- [`AttributeKeyViewParts`] for values used as dynamic attribute names: `(name)="value"`.
- [`AttributeViewParts`] for values that emit one or more full attributes in APIs that accept complete attribute fragments.
- [`ElementNameViewParts`] for values used as dynamic element names: `<(name)>...</(name)>`.

Each trait method receives a [`PartsWriter`] for the position being filled. Everything pushed through its `push_*` methods is escaped or validated for that position when the view renders; [`push_str_unescaped`][PartsWriter::push_str_unescaped] is the only opt-out and must only be given trusted markup.

For example, a type can opt into child-node rendering by implementing [`NodeViewParts`]:

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn example() -> Result {
use topcoat::{context::Cx, view::{NodeViewParts, PartsWriter}};

struct Badge(String);

impl NodeViewParts for Badge {
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_str(self.0);
    }
}

view! {
    <p>(Badge("New".to_owned()))</p>
}
# }
```

For attribute values, implement [`AttributeValueViewParts`]. Its [`attribute_present`][AttributeValueViewParts::attribute_present] method controls whether the containing attribute is rendered at all.

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn example() -> Result {
use topcoat::{context::Cx, view::{AttributeValueViewParts, PartsWriter}};

struct DataId(Option<String>);

impl AttributeValueViewParts for DataId {
    fn attribute_present(&self) -> bool {
        self.0.is_some()
    }

    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        if let Some(value) = self.0 {
            parts.push_str(value);
        }
    }
}

view! {
    <article data-id=(DataId(Some("post-1".to_owned())))></article>
}
# }
```

[`AttributeKeyViewParts`]: trait.AttributeKeyViewParts.html
[`AttributeValueViewParts`]: trait.AttributeValueViewParts.html
[AttributeValueViewParts::attribute_present]: trait.AttributeValueViewParts.html#tymethod.attribute_present
[`AttributeViewParts`]: trait.AttributeViewParts.html
[`ElementNameViewParts`]: trait.ElementNameViewParts.html
[`NodeViewParts`]: trait.NodeViewParts.html
[`PartsWriter`]: struct.PartsWriter.html
[PartsWriter::push_str_unescaped]: struct.PartsWriter.html#method.push_str_unescaped
[`component`]: attr.component.html
[`attributes!`]: macro.attributes.html
[`class!`]: macro.class.html
[`false`]: https://doc.rust-lang.org/std/keyword.false.html
[`None`]: https://doc.rust-lang.org/std/option/enum.Option.html#variant.None
[`Some`]: https://doc.rust-lang.org/std/option/enum.Option.html#variant.Some
[`topcoat::view::Attributes`]: struct.Attributes.html
[`topcoat::view::Class`]: struct.Class.html
[`view!`]: macro.view.html
