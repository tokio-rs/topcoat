The [`class!`] macro builds a [`topcoat::view::Class`] value by joining HTML class names with spaces.

Use it in the value position of a `class` attribute when the list mixes static and conditional parts:

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
