The [`class!`] macro builds a [`topcoat::view::Class`] value by joining HTML class names with spaces.

Use it to combine base classes, variants, optional classes, conditional classes, and forwarded classes. It handles the spaces between entries, so callers do not need to format or concatenate a class list. Keep a fixed class list as an ordinary `class="btn btn-lg"` attribute.

Pass the result directly as the value of a `class` attribute:

```rust
# #[topcoat::view::component]
# async fn example() -> topcoat::Result<impl topcoat::view::View> {
use topcoat::view::{View, class, view};

let is_active = true;

Ok(view! {
    <button class=(class!("btn", "btn-lg", "active" if is_active))>"Save"</button>
})
# }
```

# Syntax

The body is a comma-separated list of entries. Each entry is a Rust expression, optionally followed by a trailing condition:

- `expr` includes the entry unconditionally.
- `expr if cond` includes the entry only when `cond` is true.
- `expr if cond else alt` includes `expr` when `cond` is true and `alt` otherwise.

```rust
# #[topcoat::view::component]
# async fn example() -> topcoat::Result<impl topcoat::view::View> {
use topcoat::view::{View, class, view};

let variant: Option<&str> = Some("primary");
let sizes = vec!["px-4".to_owned(), "py-2".to_owned()];
let enabled = false;

Ok(view! {
    <button class=(class!(
        "btn",
        variant,
        sizes,
        "cursor-pointer" if enabled else "opacity-50",
    ))>"Save"</button>
})
# }
```

Entries must implement [`ClassViewParts`]. This includes strings, optional values, and collections of entries. Implement the trait to use your own types.

# Absent entries

Absent entries contribute no text or separator. This includes `None`, empty strings, and entries with a false condition. When every entry is absent, the element omits the `class` attribute:

```rust
# #[topcoat::view::component]
# async fn example() -> topcoat::Result<impl topcoat::view::View> {
use topcoat::view::{View, class, view};

let variant: Option<&str> = None;

// Renders `<p></p>`.
Ok(view! {
    <p class=(class!(variant, "active" if false))></p>
})
# }
```

# Forwarded classes

When a component accepts an attribute collection, remove its `class` entry and combine it with the component's classes. Forward the remaining attributes separately:

```rust
use topcoat::{
    Result,
    view::{Attributes, Child, View, class, component, view},
};

#[component]
async fn panel(#[default] mut attrs: Attributes, #[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        <section class=(class!("panel", attrs.remove("class"))) (attrs)>
            (child)
        </section>
    })
}
```

# Static class lists

Use [`StaticClass`] to store a literal class list in a constant:

```rust
use topcoat::view::{StaticClass, class};

const BUTTON: StaticClass = class!("btn btn-lg rounded");
```

A class list's type depends on its entries. Use a `let` binding to let Rust infer the type when the list contains expressions.

[`Attributes`]: struct.Attributes.html
[`Class`]: struct.Class.html
[`StaticClass`]: type.StaticClass.html
[`ClassViewParts`]: trait.ClassViewParts.html
[`AttributeValue`]: enum.AttributeValue.html
[`class!`]: macro.class.html
[`topcoat::view::Class`]: struct.Class.html
