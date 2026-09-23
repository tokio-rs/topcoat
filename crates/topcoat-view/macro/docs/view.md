The [`view!`] macro is Topcoat's HTML templating syntax. It stays close to real HTML instead of inventing a Rust-shaped dialect:

- HTML elements use their real names, and custom elements with dashes like `<my-widget>` work too.
- HTML void elements, such as `<br>`, `<hr>`, and `<img>`, are written without closing tags.
- All other elements need a matching closing tag, unless they are written self-closing like `<path d="..." />`, which renders exactly as written. This is mainly useful for SVG.
- Attribute names can contain `-`, `:`, and `.`, as in `data-post-id`, `aria-label`, `xmlns:xlink`, `hx-get`, and `class.active`.
- Rust keywords are valid attribute names, so `type="button"` and `for="email"` work as expected.

The main difference from HTML is that text must be quoted. A Rust macro cannot see the exact source text between tokens, so unquoted text would lose its spacing and punctuation.

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
            <my-widget data-widget-id="profile"></my-widget>
        </body>
    </html>
})
# }
```

Each attribute name can appear only once per element. Writing the same literal name twice is a compile error.

# Rust Expressions

Wrap a Rust expression in parentheses to insert it into the markup.

In child position, the expression becomes a node. Text from an expression is HTML-escaped, and a value that is itself a view, like the result of another `view!`, renders in place:

```rust
# use topcoat::{Result, view::*};
# struct User { name: &'static str }
# #[component]
# async fn example() -> Result<impl View> {
# let user = User { name: "Ada" };
let sidebar = view! { <aside></aside> };

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

The same syntax works for attribute names and element names. For an element name, the closing tag repeats the expression, but it is only evaluated once:

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

# Control Flow

[`view!`] supports Rust's `if`, `for`, `match`, and `let` with markup in their bodies. Each works both between child nodes and inside an element's opening tag, where the bodies contain attributes instead of nodes.

## `if`

Use `if`, `else if`, and `else` to choose which markup is rendered. `if let` works as well.

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

Inside an opening tag, each branch contains attributes:

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

Use `for pat in expr { ... }` to render the body once for each item:

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

Inside an opening tag, a loop adds zero or more attributes. This is useful when the attributes already exist as data:

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

When a loop renders components or live regions, give it a key. See [Keys](#keys) below.

## `match`

Use `match` to choose markup by pattern. Arms can have guards.

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

An arm body is a single node. Wrap several sibling nodes in braces:

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

Inside an opening tag, each arm contains an attribute:

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

Use `let pat = expr;` to bind a value for the nodes that follow it in the same body. `let ... else` is not supported.

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

Inside an opening tag, the binding is in scope for the attributes that follow it:

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

Call a component inside [`view!`] like a function, but with named arguments written as `name: value` and separated by commas. If the component has a `child` parameter, any view nodes after the named arguments become its child content. The child nodes are not separated by commas:

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
        // Named argument:
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

The child nodes are shorthand for a `child` argument holding a view:

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

Leaving out a required argument is a compile error. See the [`component`] macro guide for how to define components.

## Keys

Each component call gets a context with a stable identity. The identity comes from the chain of call sites leading to it in the code, so it is the same from one render to the next. The framework uses it to attach per-call data, such as state, to the call.

A `for` loop repeats its body at the same call site, so its iterations cannot be told apart by location. Add `#[key(expr)]` to the loop to give each iteration its own identity. The components called inside the iteration inherit it:

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

The key expression runs once per iteration and can use the loop's bindings. Pick a value that identifies the item, such as its database id, so the identity follows the item when the list is reordered. Any value implementing [`IdentityKey`] works as a key.

The key is optional. A loop without one still renders, but its iterations share an ambiguous identity. Reading that identity through a component's context fails with a message naming the loop that is missing its key. A keyed loop nested in an unkeyed loop is ambiguous as well.

The macro only sets up identities for components and live regions. Ordinary Rust expressions use whatever context you pass them. If a helper function needs a distinct identity for each iteration, pass it a context derived with [`Cx::keyed`](../context/struct.Cx.html#method.keyed), such as `helper(&cx.keyed(item.id))`.

# Views Are Lazy

A [`view!`] expression does not render where it is written. It evaluates to a value implementing [`View`], and the expressions inside it run when that view renders: when it becomes a response, or when the view it is inserted into renders. A view that never renders never runs them, just like a future that is never awaited.

A view behaves like an `async move` block. It moves every variable the template mentions into itself:

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

When you need a value both inside the view and after it, insert a clone instead.

A view that captures a reference borrows the data behind it, so it cannot outlive that data. In practice this rarely gets in the way. Component props and anything borrowed from the request context live until the render is over, so references like a `&str` prop are safe to use in a view.

# Concurrent Rendering

The components inside a [`view!`] render concurrently. Sibling components, the iterations of a `for` loop, and the taken branch of an `if` or `match` all start at the same time. A component that waits on a database query or an HTTP request does not hold up the rest of the view, which avoids request waterfalls.

The rendered markup always appears in source order, no matter which component finishes first. The order in which component bodies and template expressions run is not specified and can change between renders.

So treat component bodies and template expressions as computations without side effects. Do not rely on another component or expression in the same view having run first, and do not communicate through shared mutable state. If work has to happen in a particular order, do it before building the view and insert the results.

# Boolean And Conditional Attributes

[Boolean HTML attributes](https://developer.mozilla.org/en-US/docs/Glossary/Boolean/HTML) such as `disabled`, `required`, and `checked` are true when present and false when absent. HTML expects a present boolean attribute to have an empty value.

When the value is known where you write the view, prefer the literal form `disabled=""` over the expression form `disabled=(true)`. Both render as `disabled=""`, but the literal is part of the static markup that the macro prepares at compile time, while `(true)` is an expression evaluated on every render.

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn example() -> Result<impl View> {
Ok(view! {
    <input type="email" required="" disabled="">
})
# }
```

When the value is only known at run time, use an expression. An expression value can remove its attribute: when it evaluates to [`false`] or [`None`], the whole attribute is left out, and [`true`] renders the attribute with an empty value. A [`bool`] expression therefore gives a boolean attribute the presence behavior HTML expects, and an [`Option`] does the same for attributes that carry a value:

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

The rendered opening tag includes `aria-current="page"` and leaves out `disabled` and `title` completely.

Only expression values can remove an attribute. A literal attribute is always present:

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn example() -> Result<impl View> {
Ok(view! {
    <button disabled="false">"Still disabled in HTML"</button>
})
# }
```

Some attributes, such as `aria-expanded` and `contenteditable`, take the strings `"true"` and `"false"` as values. These are enumerated attributes, not boolean attributes: `"false"` means something different from leaving the attribute out. Pass them strings instead of booleans:

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

Two companion macros build attribute values outside of an element:

- [`attributes!`] uses the same attribute syntax as [`view!`] to build an [`Attributes`] collection. Insert it into an element as `<div (attrs)>`, or pass it to a component.
- [`class!`] builds a [`Class`] list for a `class` attribute from static and conditional entries. It separates the entries with single spaces and leaves out the attribute when no entry is present.

# Client Reactivity

A view can also carry client-side behavior: `$(...)` runtime expressions, `@event` handlers, and `:name` bind attributes. The [runtime guide](../runtime/index.html) explains them.

# Status Codes And Response Headers

A view can set the status code and headers of the HTTP response it renders into. A [`StatusCode`] in node position sets the response status, and a [`HeaderMap`] or a single `(HeaderName, HeaderValue)` pair adds response headers. None of them render any content.

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

When several parts of a render declare these, render order decides. The first status code rendered wins. For each header name, the first part that mentions it provides all of that name's values. So in a layout, the placement decides who wins: a declaration placed before the layout's slot overrides whatever the page declares, and one placed after the slot is a fallback that the page can override:

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

Every page under `/docs` now gets `Cache-Control: max-age=60`, unless it declares its own `Cache-Control`.

A status code in node position never renders as text. To display one, render one of its accessors instead, such as `(status.as_u16())`.

These declarations require the `router` feature (or the `http` feature of the `topcoat-view` crate). They take effect when the rendered view becomes a response. Rendering a view to a plain string ignores them.

# Rendering Outside A Component

Inside a [`component`], `#[page]`, `#[layout]`, or `#[shard]`, the request context is in scope implicitly, so `view!` can call components and use reactive markup without extra setup. In a plain function, pass the context at the start of the `view!` body:

```rust
# use topcoat::{Result, context::Cx, view::*};
# #[component]
# async fn greeting(name: &str) -> Result<impl View> { Ok(view! { <h1>(name)</h1> }) }
async fn render(cx: &Cx) -> Result<impl View> {
    Ok(view! { cx => greeting(name: "World") })
}
```

With `cx =>`, the view holds its own clone of the context, so it does not borrow `cx`.

A view behaves the same whether or not it names its context. As the outermost view of a render it produces self-contained content that can become a response on its own. Inserted into another view, it renders as part of that view.

# Custom Values In Markup

[`view!`] accepts Rust values through a small set of traits, one per position. Implement the trait for each position where your type should be accepted:

- [`NodeViewParts`] for child nodes: `(value)`.
- [`AttributeValueViewParts`] for attribute values: `name=(value)`.
- [`AttributeKeyViewParts`] for attribute names: `(name)="value"`.
- [`AttributeViewParts`] for values that add whole attributes to an element: `<div (value)>`.
- [`ElementNameViewParts`] for element names: `<(name)>...</(name)>`.

Each trait method receives a [`PartsWriter`] for the position being filled. Everything pushed through its `push_*` methods is escaped or validated for that position when the view renders. The `push_*_unescaped` methods are the only way around this, and must only be given trusted markup.

For example, a type can render as a child node by implementing [`NodeViewParts`]:

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

For attribute values, implement [`AttributeValueViewParts`]. Its [`attribute_present`][AttributeValueViewParts::attribute_present] method decides whether the attribute is rendered at all:

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
[`Attributes`]: struct.Attributes.html
[`Class`]: struct.Class.html
[`ElementNameViewParts`]: trait.ElementNameViewParts.html
[`NodeViewParts`]: trait.NodeViewParts.html
[`PartsWriter`]: struct.PartsWriter.html
[`attributes!`]: macro.attributes.html
[`class!`]: macro.class.html
[`component`]: attr.component.html
[`view!`]: macro.view.html
[`View`]: trait.View.html
[`IdentityKey`]: ../core/identity/trait.IdentityKey.html
[`bool`]: https://doc.rust-lang.org/std/primitive.bool.html
[`false`]: https://doc.rust-lang.org/std/keyword.false.html
[`true`]: https://doc.rust-lang.org/std/keyword.true.html
[`None`]: https://doc.rust-lang.org/std/option/enum.Option.html#variant.None
[`Option`]: https://doc.rust-lang.org/std/option/enum.Option.html
[`StatusCode`]: https://docs.rs/http/latest/http/status/struct.StatusCode.html
[`HeaderMap`]: https://docs.rs/http/latest/http/header/struct.HeaderMap.html
