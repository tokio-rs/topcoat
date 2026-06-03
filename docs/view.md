# The `view!` macro

The `view!` macro is Topcoat's HTML templating syntax. It tries to be unsurprising by staying close to real HTML instead of inventing a Rust-shaped HTML dialect.

That means:

- HTML elements use their real names.
- HTML void elements, such as `<br>`, `<hr>`, `<img>`, `<input>`, `<meta>`, and `<link>`, are written without closing tags.
- Non-void elements need matching closing tags.
- Attribute names can use HTML separators like `-`, `:`, and `.`: `data-post-id`, `aria-label`, `xmlns:xlink`, `hx-get`, `class.active`.
- Rust keywords are still valid HTML attribute names, so `type="button"` and `for="email"` work as expected.
- Literal text and literal attribute values are string literals.

```rust
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
```

Element names can use dashes, so custom elements fit naturally:

```rust
view! {
    <my-widget data-widget-id="profile"></my-widget>
}
```

## Rust Expressions

Use parentheses to interpolate a Rust expression into markup.

In child position, the expression becomes a node:

```rust
view! {
    <h1>"Hello, " (user.name) "!"</h1>
    (sidebar)
}
```

In attribute value position, the expression becomes the value:

```rust
view! {
    <a href=(url) aria-current=(is_current)>"Open"</a>
}
```

The same parenthesized expression syntax can also be used for dynamic attribute names and dynamic element names:

```rust
let tag = "section";
let attr = "data-state";

view! {
    <(tag) (attr)="ready">"Loaded"</(tag)>
}
```

Literal text must be quoted because unquoted Rust identifiers are meaningful to the macro:

```rust
view! {
    <p>"This is text"</p>
    <p>(computed_text)</p>
}
```

## Control Flow

Control flow in `view!` is Rust control flow with markup bodies. The macro lowers these constructs into ordinary Rust statements that append to the view being built.

### `if`

Use `if`, `else if`, and `else` to choose which markup is emitted.

```rust
view! {
    if user.is_some() {
        <a href="/account">"Account"</a>
    } else {
        <a href="/login">"Sign in"</a>
    }
}
```

In attributes, each branch emits attributes instead of child nodes:

```rust
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
```

### `for`

Use `for pat in expr { ... }` to render the body once for each item.

```rust
view! {
    <ul>
        for post in posts {
            <li>
                <a href=(post.url)>(post.title)</a>
            </li>
        }
    </ul>
}
```

In attributes, a loop can emit zero or more attributes. This is useful when you already have attributes represented as data:

```rust
view! {
    <div
        for (name, value) in attrs {
            (name)=(value)
        }
    ></div>
}
```

### `match`

Use `match` to choose markup from patterns. Match arms can also use guards.

```rust
view! {
    match status {
        Status::Draft => <span>"Draft"</span>,
        Status::Published { title } => <a href="/posts">(title)</a>,
        Status::Archived if show_archived => <span>"Archived"</span>,
        _ => "",
    }
}
```

A match arm body is one view node. If a branch needs multiple sibling nodes, wrap them in a block:

```rust
view! {
    match user {
        Some(user) => {
            <h1>(user.name)</h1>
            <p>"Signed in"</p>
        },
        None => <a href="/login">"Sign in"</a>,
    }
}
```

In attributes, each arm emits one attribute node:

```rust
view! {
    <article
        match state {
            State::Open => class="open",
            State::Closed => aria-disabled=(true),
        }
    ></article>
}
```

For multiple conditional attributes, put the `if`, `for`, or `match` at the level where it can emit the attributes you need.

### `let`

Use `let pat = expr;` to bind values for later nodes in the same body.

```rust
view! {
    <article>
        let title = post.title.trim();

        <h1>(title)</h1>
        <a href=(post.url)>"Read"</a>
    </article>
}
```

The same works in an attribute list. The binding is in scope for attributes that follow it:

```rust
view! {
    <a
        let href = post.url();
        href=(href)
        data-slug=(post.slug)
    >
        (post.title)
    </a>
}
```

## Components

Components are called inside `view!` with function-call syntax. Named arguments use `name: value`, and child nodes can be passed after the named arguments:

```rust
view! {
    panel(
        title: "Profile",
        <p>"Account details"</p>
        badge(
            label: "Active",
            tone: "success",
        )
    )
}
```

All component parameters are named parameters, except `child`, which can be passed unnamed in the last position. Conceptually, those trailing child nodes are the same thing as a `child` parameter whose value is a `view! { ... }` containing those nodes.

See [The `component` macro](component.md) for defining components and passing child content.

## Conditional Attributes

Expression attributes can remove themselves from the rendered markup.

When an attribute value evaluates to `false`, the whole attribute is omitted. When it evaluates to `None`, the whole attribute is omitted. `Some(value)` renders the attribute using the inner value.

```rust
view! {
    <button
        disabled=(is_disabled)
        aria-current=(is_current.then_some("page"))
        title=(maybe_title)
    >
        "Save"
    </button>
}
```

If the values are:

```rust
let is_disabled = false;
let is_current = true;
let maybe_title: Option<&str> = None;
```

then the rendered opening tag includes `aria-current="page"`, but leaves out `disabled` and `title` completely.

This omission logic applies to expression attributes. Literal attributes are always present:

```rust
view! {
    <button disabled="false">"Still disabled in HTML"</button>
}
```

For reusable runtime attribute collections, use [the `attributes!` macro](attributes.md). It accepts the same attribute syntax and creates a `topcoat::view::Attributes` value that can be passed around and inserted into an element as an attribute fragment.

## Custom Values In Markup

The macro accepts dynamic Rust values by routing them through small runtime traits. Implement the trait for the position where your type should be accepted:

- `NodeViewParts` for values used as child nodes: `(value)`.
- `AttributeValueViewParts` for values used as attribute values: `name=(value)`.
- `AttributeKeyViewParts` for values used as dynamic attribute names: `(name)="value"`.
- `AttributeViewParts` for values that emit one or more full attributes in APIs that accept complete attribute fragments.
- `ElementNameViewParts` for values used as dynamic element names: `<(name)>...</(name)>`.

For example, a type can opt into child-node rendering by implementing `NodeViewParts`:

```rust
use topcoat::view::{NodeViewParts, ViewParts};

struct Badge(String);

impl NodeViewParts for Badge {
    fn into_view_parts(self, parts: &mut ViewParts) {
        parts.push(self.0);
    }
}

view! {
    <p>(Badge("New".to_owned()))</p>
}
```

For attribute values, implement `AttributeValueViewParts`. Its `attribute_present` method controls whether the containing attribute is rendered at all.

```rust
use topcoat::view::{AttributeValueViewParts, ViewParts};

struct DataId(Option<String>);

impl AttributeValueViewParts for DataId {
    fn attribute_present(&self) -> bool {
        self.0.is_some()
    }

    fn into_view_parts(self, parts: &mut ViewParts) {
        if let Some(value) = self.0 {
            parts.push(value);
        }
    }
}

view! {
    <article data-id=(DataId(Some("post-1".to_owned())))></article>
}
```
