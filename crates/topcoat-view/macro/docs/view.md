The [`view!`] macro is Topcoat's HTML templating syntax. It tries to be unsurprising by staying close to real HTML instead of inventing a Rust-shaped HTML dialect. That means:

- HTML elements use their real names.
- HTML void elements, such as `<br>`, `<hr>`, and `<img>`, are written without closing tags.
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
# let sidebar = view! { <aside></aside> };
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
            State::Closed => aria-disabled="true",
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

## Keys

Each component invocation has a stable identity derived from the chain of invocation sites leading down to it in code, the same from one render to the next. The framework attaches per-invocation data such as state to it. Inside a `for` body one component invocation renders many times, and the site alone cannot tell the repetitions apart. Use the reserved `key` property on component to distinguish individual calls inside of the loop:

```rust
# use topcoat::{Result, view::*};
# struct Post { id: u64, title: &'static str }
# #[component]
# async fn post_card(title: &str) -> Result { view! { <article>(title)</article> } }
# #[component]
# async fn example() -> Result {
# let posts = vec![Post { id: 1, title: "A" }];
view! {
    for post in posts {
        post_card(key: post.id, title: post.title)
    }
}
# }
```

Key the invocation with a value that identifies the item behind it, such as its database id, not the loop index: the identity then follows the item when the list reorders. Any value implementing [`IdentityKey`] works as a key. A `key:` is also allowed outside a loop, for an invocation that repeats in ways the macro cannot see.

A repeated invocation without a `key:` still renders, but its identity is ambiguous. Consuming an ambiguous identity, in the component itself or anywhere nested below it, errors with the location of the invocation that is missing its key.

# Concurrent Rendering

The components inside a [`view!`] render concurrently. Sibling components, the iterations of a `for` loop, the taken branch of an `if` or `match`, a component and the child nodes passed to it, and components nested at any depth all start at the same time. A component waiting on a database query or an HTTP request therefore does not hold up the rest of the view, which avoids request waterfalls.

The rendered markup always appears in source order, no matter which component finishes first. What is unspecified is the order in which component bodies run, and that order can change between renders. Treat a [`view!`] body as a set of functions without side effects: a component takes its props, reads the request context, and returns markup. Do not rely on another component in the same view having run first, and do not communicate between components through shared mutable state.

Plain Rust in the view, such as interpolated expressions, `let` bindings, loop iterators, and branch conditions, still runs in source order. Only the components render concurrently.

# Reactive Markup

A `live` construct consumes a reactive expression and re-renders its arms in place when the expression's state changes; nothing else on the page runs again. The first reactive expression is [`defer`], which wraps a slow future instead of awaiting it. Its states are [`Deferred::Pending`] now and [`Deferred::Ready`] once, when the future completes, so a page renders a skeleton immediately and swaps in the real content when the data arrives:

```rust
# use topcoat::{Result, context::Cx, view::*};
# struct Drink { slug: String, title: String }
# async fn drinks(_cx: &Cx) -> Result<Vec<Drink>> { Ok(Vec::new()) }
# #[component]
# async fn drink_card(drink: Drink) -> Result { view! { <p>(drink.title)</p> } }
# #[component]
# async fn example(cx: &Cx) -> Result {
view! {
    <div class="grid">
        live match defer(drinks(cx)) {
            Deferred::Pending => {
                <div class="skeleton"></div>
            }
            Deferred::Ready(drinks) => {
                for drink in drinks? {
                    drink_card(key: &drink.slug, drink: drink)
                }
            }
        }
    </div>
}
# }
```

The macro supplies `defer`'s context argument inside a view; outside one, call [`defer`] with the context explicitly. `live if let` fits when the missing state should render nothing. Both arms are full view scopes with access to the surrounding function's locals, and a `?` inside an arm is a real `?`: the failure fails the construct and climbs to the enclosing scope.

Because the arms may run again, an arm can borrow values from the surrounding function but not consume them; a prop that takes ownership gets an explicit `.clone()`.

A component invocation and a `view!` expression both evaluate to a [`ViewHandle`], a reactive expression whose states are the component's declared `Result`. Interpolating the handle splices the content and lets its errors bubble; a `live match` on it catches them in place, which is how a layout owns the error page for whatever it wraps:

```rust
# use topcoat::{Result, router::error::NotFoundError, view::*};
# #[component]
# async fn drink_grid() -> Result { view! { <p>"grid"</p> } }
# #[component]
# async fn example() -> Result {
view! {
    live match drink_grid() {
        Ok(grid) => { (grid) }
        Err(error) if error.downcast_ref::<NotFoundError>().is_some() => {
            <p>"The menu is unavailable right now."</p>
        }
        other => { (other?) }
    }
}
# }
```

A `live` construct needs the live render a `#[component]`, `#[page]`, or `#[layout]` provides; anywhere else it does not compile:

```compile_fail
# use topcoat::{Result, context::Cx, view::*};
async fn render(cx: &Cx) -> Result {
    view! { cx =>
        live if let Deferred::Ready(n) = defer(std::future::ready(1)) {
            (n)
        }
    }
}
```

# Boolean And Conditional Attributes

[Boolean HTML attributes](https://developer.mozilla.org/en-US/docs/Glossary/Boolean/HTML) such as `disabled`, `required`, and `checked` are true when the attribute is present and false when it is absent. HTML expects a present boolean attribute to have an empty value.

When the value is known where the view is written, prefer the literal form `disabled=""` over the expression form `disabled=(true)`. Both render as `disabled=""`, but the literal is static markup that the macro folds into the pre-rendered parts of the template, while `(true)` is a Rust expression evaluated on every render.

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn example() -> Result {
view! {
    <input type="email" required="" disabled="">
}
# }
```

When the value is only known at run time, pass an expression. Expression attributes can remove themselves from the rendered markup: when the value evaluates to [`false`] or [`None`], the whole attribute is omitted, while a [`true`] value renders the attribute with an empty value. A [`bool`] expression therefore gives a boolean attribute exactly the presence behavior HTML expects, and [`Some`]/[`None`] extend the same logic to attributes that carry values:

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

Attributes that take the literal strings `"true"` and `"false"` as values, such as `aria-expanded` or `contenteditable`, are enumerated attributes, not boolean attributes. For them, `"false"` means something different than omitting the attribute, so pass strings instead of booleans:

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn example() -> Result {
# let expanded = false;
view! {
    <button aria-expanded=(if expanded { "true" } else { "false" })>"Menu"</button>
}
# }
```

# Attribute Collections And Class Lists

Two companion macros build attribute values outside a view:

- [`attributes!`] uses the same attribute syntax as [`view!`] to build a reusable [`topcoat::view::Attributes`] collection that inserts into an element as an attribute fragment.
- [`class!`] assembles a `class` attribute value from static and conditional entries into a [`topcoat::view::Class`], which joins its entries with single spaces and omits the attribute entirely when no entry is present.

# Status Codes And Response Headers

A view can declare the status code and headers of the HTTP response it renders into. A [`StatusCode`] in node position sets the response status, and a [`HeaderMap`] or a single `(HeaderName, HeaderValue)` pair adds response headers. None of them render any content.

```rust
# use topcoat::{Result, view::*};
# use topcoat::router::{StatusCode, HeaderValue, header};
# #[component]
# async fn example() -> Result {
view! {
    (StatusCode::NOT_FOUND)
    ((header::CACHE_CONTROL, HeaderValue::from_static("no-store")))
    <h1>"Page not found"</h1>
}
# }
```

Competing declarations resolve by render order: the first status code rendered wins, and for each header name the first part that mentions it provides all of that name's values. Placement therefore decides precedence between a layout and the pages it wraps. A declaration placed before the layout's slot overrides whatever the page declares; placed after the slot it is a fallback the page can override:

```rust
# use topcoat::{Result, view::*};
# use topcoat::router::{HeaderValue, header, layout};
#[layout("/docs")]
async fn docs_layout(slot: ViewHandle<'_>) -> Result {
    view! {
        <main>(slot)</main>
        ((header::CACHE_CONTROL, HeaderValue::from_static("max-age=60")))
    }
}
```

Every page under `/docs` now gets `Cache-Control: max-age=60` unless it declares its own `Cache-Control`.

A status code in node position never renders text. To display one, render one of its accessors instead, such as `(status.as_u16())`.

These declarations require the `router` feature (or the `topcoat-view` crate's `http` feature) and take effect when the rendered view becomes a response; rendering a view to a plain string discards them.

# Rendering Outside A Component

Inside a [`component`], `#[page]`, `#[layout]`, or `#[shard]`, the request context is in scope implicitly, so `view!` can render components and reactive markup with no ceremony. In a plain function you pass it explicitly at the start of the `view!` macro:

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

Each trait method receives a [`PartsWriter`] for the position being filled. Everything pushed through its `push_*` methods is escaped or validated for that position when the view renders; the `push_*_unescaped` methods are the only opt-out and must only be given trusted markup.

For example, a type can opt into child-node rendering by implementing [`NodeViewParts`]:

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn example() -> Result {
use topcoat::{context::Cx, view::{NodeViewParts, PartsWriter}};

struct Badge(String);

impl NodeViewParts for Badge {
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_string(self.0);
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
            parts.push_string(value);
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
[`component`]: attr.component.html
[`memoize`]: https://docs.rs/topcoat/latest/topcoat/context/attr.memoize.html
[`attributes!`]: macro.attributes.html
[`class!`]: macro.class.html
[`bool`]: https://doc.rust-lang.org/std/primitive.bool.html
[`false`]: https://doc.rust-lang.org/std/keyword.false.html
[`true`]: https://doc.rust-lang.org/std/keyword.true.html
[`None`]: https://doc.rust-lang.org/std/option/enum.Option.html#variant.None
[`Some`]: https://doc.rust-lang.org/std/option/enum.Option.html#variant.Some
[`topcoat::view::Attributes`]: struct.Attributes.html
[`topcoat::view::Class`]: struct.Class.html
[`view!`]: macro.view.html
[`Identity`]: identity/struct.Identity.html
[`Identity::current`]: identity/struct.Identity.html#method.current
[`IdentityKey`]: identity/trait.IdentityKey.html
[`StatusCode`]: https://docs.rs/http/latest/http/status/struct.StatusCode.html
[`HeaderMap`]: https://docs.rs/http/latest/http/header/struct.HeaderMap.html
[`defer`]: fn.defer.html
[`Deferred::Pending`]: enum.Deferred.html#variant.Pending
[`Deferred::Ready`]: enum.Deferred.html#variant.Ready
[`ViewHandle`]: struct.ViewHandle.html
