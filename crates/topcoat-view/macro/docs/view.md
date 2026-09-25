The [`view!`] macro builds HTML from markup and Rust expressions. It follows HTML syntax with a few rules:

- HTML elements use their real names.
- HTML void elements, such as `<br>`, `<hr>`, and `<img>`, are written without closing tags.
- Non-void elements need matching closing tags.
- Attribute names can use HTML separators like `-`, `:`, and `.`: `data-post-id`, `aria-label`, `xmlns:xlink`, `hx-get`, `class.active`.
- Rust keywords are still valid HTML attribute names, so `type="button"` and `for="email"` work as expected.

Write text nodes in quotes.

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn example() -> Result<impl View> {
Ok(view! {
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
})
# }
```

Element names can contain dashes for custom elements:

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn example() -> Result<impl View> {
Ok(view! {
    <my-widget data-widget-id="profile"></my-widget>
})
# }
```

# Rust Expressions

Use parentheses to interpolate a Rust expression into markup.

In child position, the expression becomes a node:

```rust
# use topcoat::{Result, view::*};
# struct User { name: &'static str }
# #[component]
# async fn example() -> Result<impl View> {
# let user = User { name: "Ada" };
# let sidebar = view! { <aside></aside> };
Ok(view! {
    <h1>"Hello, " (user.name) "!"</h1>
    (sidebar)
})
# }
```

In attribute value position, the expression becomes the value:

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn example() -> Result<impl View> {
# let url = "/posts";
# let is_current = true;
Ok(view! {
    <a href=(url) aria-current=(is_current)>"Open"</a>
})
# }
```

Use parentheses for dynamic attribute and element names too:

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn example() -> Result<impl View> {
let tag = "section";
let attr = "data-state";

Ok(view! {
    <(tag) (attr)="ready">"Loaded"</(tag)>
})
# }
```

Use a quoted string for literal text or an expression for computed text:

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn example() -> Result<impl View> {
# let computed_text = "Computed";
Ok(view! {
    <p>"This is text"</p>
    <p>(computed_text)</p>
})
# }
```

# Control Flow

Use Rust control flow with markup in its branches and loop bodies.

## `if`

Use `if`, `else if`, and `else` to choose which markup is emitted.

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn example() -> Result<impl View> {
# let user: Option<()> = None;
Ok(view! {
    if user.is_some() {
        <a href="/account">"Account"</a>
    } else {
        <a href="/login">"Sign in"</a>
    }
})
# }
```

In attributes, each branch emits attributes instead of child nodes:

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn example() -> Result<impl View> {
# let current = true;
Ok(view! {
    <a
        href="/posts"
        if current {
            aria-current="page"
            class="active"
        }
    >
        "Posts"
    </a>
})
# }
```

## `for`

Use `for pat in expr { ... }` to render the body once for each item.

```rust
# use topcoat::{Result, view::*};
# struct Post { url: &'static str, title: &'static str }
# #[component]
# async fn example() -> Result<impl View> {
# let posts = vec![Post { url: "/a", title: "A" }];
Ok(view! {
    <ul>
        for post in posts {
            <li>
                <a href=(post.url)>(post.title)</a>
            </li>
        }
    </ul>
})
# }
```

In attributes, a loop can emit zero or more attributes. This is useful when you already have attributes represented as data:

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn example() -> Result<impl View> {
# let attrs = vec![("data-id", "1")];
Ok(view! {
    <div
        for (name, value) in attrs {
            (name)=(value)
        }
    ></div>
})
# }
```

## `match`

Use `match` to choose markup from patterns. Match arms can also use guards.

```rust
# use topcoat::{Result, view::*};
# enum Status { Draft, Published { title: &'static str }, Archived }
# #[component]
# async fn example() -> Result<impl View> {
# let status = Status::Draft;
# let show_archived = true;
Ok(view! {
    match status {
        Status::Draft => <span>"Draft"</span>,
        Status::Published { title } => <a href="/posts">(title)</a>,
        Status::Archived if show_archived => <span>"Archived"</span>,
        _ => "",
    }
})
# }
```

A match arm body is one view node. If a branch needs multiple sibling nodes, wrap them in a block:

```rust
# use topcoat::{Result, view::*};
# struct User { name: &'static str }
# #[component]
# async fn example() -> Result<impl View> {
# let user: Option<User> = None;
Ok(view! {
    match user {
        Some(user) => {
            <h1>(user.name)</h1>
            <p>"Signed in"</p>
        },
        None => <a href="/login">"Sign in"</a>,
    }
})
# }
```

In attributes, each arm can emit attribute nodes:

```rust
# use topcoat::{Result, view::*};
# enum State { Open, Closed }
# #[component]
# async fn example() -> Result<impl View> {
# let state = State::Open;
Ok(view! {
    <article
        match state {
            State::Open => class="open",
            State::Closed => aria-disabled="true",
        }
    ></article>
})
# }
```

## `let`

Use `let pat = expr;` to bind values for later nodes in the same body.

```rust
# use topcoat::{Result, view::*};
# struct Post { title: &'static str, url: &'static str }
# #[component]
# async fn example() -> Result<impl View> {
# let post = Post { title: " Hello ", url: "/hello" };
Ok(view! {
    <article>
        let title = post.title.trim();

        <h1>(title)</h1>
        <a href=(post.url)>"Read"</a>
    </article>
})
# }
```

The same works in an attribute list. The binding is in scope for attributes that follow it:

```rust
# use topcoat::{Result, view::*};
# struct Post { slug: &'static str, title: &'static str }
# impl Post { fn url(&self) -> &str { "/hello" } }
# #[component]
# async fn example() -> Result<impl View> {
# let post = Post { slug: "hello", title: "Hello" };
Ok(view! {
    <a
        let href = post.url();
        href=(href)
        data-slug=(post.slug)
    >
        (post.title)
    </a>
})
# }
```

# Components

Call components with comma-separated `name: value` arguments. If a component accepts a `child` property, place child nodes after the named arguments. Child nodes do not need commas:

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn panel(title: &str, #[default] child: Child<'_>) -> Result<impl View> { Ok(view! { <section>(title)(child)</section> }) }
# #[component]
# async fn badge(label: &str, tone: &str) -> Result<impl View> { Ok(view! { <span>(label)(tone)</span> }) }
# #[component]
# async fn example() -> Result<impl View> {
Ok(view! {
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
})
# }
```

You can also pass the child view explicitly:

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn panel(title: &str, #[default] child: Child<'_>) -> Result<impl View> { Ok(view! { <section>(title)(child)</section> }) }
# #[component]
# async fn badge(label: &str, tone: &str) -> Result<impl View> { Ok(view! { <span>(label)(tone)</span> }) }
# #[component]
# async fn example() -> Result<impl View> {
Ok(view! {
    panel(
        title: "Profile",
        // Named child parameter:
        child: view! {
            <p>"Account details"</p>
            badge(
                label: "Active",
                tone: "success",
            )
        }.into()
    )
})
# }
```

See how to define components in the [`component`] macro guide.

## Keys

Each component call has a stable identity that associates its state with the same call across renders. In a `for` loop, use `#[key(expr)]` to distinguish iterations and the components they render:

```rust
# use topcoat::{Result, view::*};
# struct Post { id: u64, title: &'static str }
# #[component]
# async fn post_card(title: &str) -> Result<impl View> { Ok(view! { <article>(title)</article> }) }
# #[component]
# async fn example() -> Result<impl View> {
# let posts = vec![Post { id: 1, title: "A" }];
Ok(view! {
    #[key(post.id)]
    for post in posts {
        post_card(title: post.title)
    }
})
# }
```

The key expression is evaluated once per iteration and can use the loop's bindings. Choose a value that identifies the item, such as its database id, so identity follows the item when the list reorders. Any value implementing [`IdentityKey`] works as a key.

The attribute is optional. An unkeyed loop still renders, but its iteration identity is ambiguous. Consuming that identity through a component's context errors with the location of the loop missing its key. Nested keyed loops inherit ambiguity from an unkeyed outer loop.

Ordinary Rust expressions use the context explicitly passed to them. The macro does not rebind context variables inside loops. If a helper needs a distinct identity for each call, pass a context derived with [`Cx::keyed`](../context/struct.Cx.html#method.keyed), such as `helper(&cx.keyed(item.id))`.

# Views Are Lazy

A [`view!`] expression creates a value implementing [`View`]. Expressions inside it run only when the view renders, such as when it becomes a response. They do not run if the view is never rendered.

A view captures variables by moving them, like an `async move` block:

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn example() -> Result<impl View> {
let title = String::from("Hello");
let header = view! { <h1>(title)</h1> };

// `title` has moved into `header` and cannot be used here anymore.
Ok(view! {
    (header)
    <p>"Welcome!"</p>
})
# }
```

If you also need a value after constructing the view, clone it into a separate binding first and use that binding in the view.

A view can capture references, but it cannot outlive the borrowed data. Component parameters such as `&str` can be used directly because they remain valid while the component renders.

# Concurrent Rendering

Components inside a [`view!`] render concurrently. While one waits for a database query or HTTP request, others can make progress.

The rendered markup always appears in source order, no matter which component finishes first. The order in which component bodies and template expressions run is unspecified and can change between renders.

Treat component bodies and template expressions as computations without side effects. Do not rely on another component or expression in the same view having run first, and do not communicate through shared mutable state. If work needs to happen in a particular order, perform it before constructing the view and interpolate the resulting values.

# Boolean And Conditional Attributes

[Boolean HTML attributes](https://developer.mozilla.org/en-US/docs/Glossary/Boolean/HTML) such as `disabled` are true when present and false when absent. Use an empty string to include one unconditionally:

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn example() -> Result<impl View> {
Ok(view! {
    <input type="email" required="" disabled="">
})
# }
```

Use a [`bool`] expression for a conditional boolean attribute. [`true`] renders an empty attribute and [`false`] omits it. For attributes with values, use [`Some`] to include the value or [`None`] to omit the attribute:

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn example() -> Result<impl View> {
let is_disabled = false;
let is_current = true;
let maybe_title: Option<&str> = None;

Ok(view! {
    <button
        disabled=(is_disabled)
        aria-current=(is_current.then_some("page"))
        title=(maybe_title)
    >
        "Save"
    </button>
})
# }
```

The rendered opening tag includes `aria-current="page"`, but leaves out `disabled` and `title` completely.

This omission logic applies to expression attributes. Literal attributes are always present:

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn example() -> Result<impl View> {
Ok(view! {
    <button disabled="false">"Still disabled in HTML"</button>
})
# }
```

Attributes that take the literal strings `"true"` and `"false"` as values, such as `aria-expanded` or `contenteditable`, are enumerated attributes, not boolean attributes. For them, `"false"` means something different than omitting the attribute, so pass strings instead of booleans:

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn example() -> Result<impl View> {
# let expanded = false;
Ok(view! {
    <button aria-expanded=(if expanded { "true" } else { "false" })>"Menu"</button>
})
# }
```

# Attribute Collections And Class Lists

Use [`attributes!`] to build an attribute collection separately from its element. Use [`class!`] to combine static and conditional class names. Their guides cover the syntax and how to insert the values into a view.

# Status Codes And Response Headers

A view can declare the status code and headers of the HTTP response it renders into. A [`StatusCode`] in node position sets the response status, and a [`HeaderMap`] or a single `(HeaderName, HeaderValue)` pair adds response headers. None of them render any content.

```rust
# use topcoat::{Result, view::*};
# use topcoat::router::{StatusCode, HeaderValue, header};
# #[component]
# async fn example() -> Result<impl View> {
Ok(view! {
    (StatusCode::NOT_FOUND)
    ((header::CACHE_CONTROL, HeaderValue::from_static("no-store")))
    <h1>"Page not found"</h1>
})
# }
```

The first status code in markup order wins. For each header name, the first declaration supplies all its values. In a layout, place declarations before the slot to override the page, or after the slot to provide defaults:

```rust
# use topcoat::{Result, view::*};
# use topcoat::router::{HeaderValue, Slot, header, layout};
#[layout("/docs")]
async fn docs_layout(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        <main>(slot)</main>
        ((header::CACHE_CONTROL, HeaderValue::from_static("max-age=60")))
    })
}
```

Every page under `/docs` gets `Cache-Control: max-age=60` unless it declares its own `Cache-Control`.

A status code in node position never renders text. To display one, render one of its accessors instead, such as `(status.as_u16())`.

These declarations require the `router` feature (or the `topcoat-view` crate's `http` feature) and take effect when the rendered view becomes a response; rendering a view to a plain string discards them.

# Rendering Outside A Component

Inside a [`component`], the request context is available implicitly. In a plain function, pass the context at the start of the macro:

```rust
# use topcoat::{Result, context::Cx, view::*};
# #[component]
# async fn greeting(name: &str) -> Result<impl View> { Ok(view! { <h1>(name)</h1> }) }
async fn render(cx: &Cx) -> Result<impl View> {
    Ok(view! { cx => greeting(name: "World") })
}
```

A view can render on its own or be interpolated into another view.

# Custom Values In Markup

To render a custom Rust type, implement the trait for the position where it will appear:

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
# async fn example() -> Result<impl View> {
use topcoat::{context::Cx, view::{NodeViewParts, PartsWriter}};

struct Badge(String);

impl NodeViewParts for Badge {
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_string(self.0);
    }
}

Ok(view! {
    <p>(Badge("New".to_owned()))</p>
})
# }
```

For attribute values, implement [`AttributeValueViewParts`]. Its [`attribute_present`][AttributeValueViewParts::attribute_present] method controls whether the containing attribute is rendered at all.

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn example() -> Result<impl View> {
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

Ok(view! {
    <article data-id=(DataId(Some("post-1".to_owned())))></article>
})
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
[`View`]: trait.View.html
[`Identity`]: ../core/identity/struct.Identity.html
[`IdentityKey`]: ../core/identity/trait.IdentityKey.html
[`StatusCode`]: https://docs.rs/http/latest/http/status/struct.StatusCode.html
[`HeaderMap`]: https://docs.rs/http/latest/http/header/struct.HeaderMap.html
