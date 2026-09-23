The [`class!`] macro joins HTML classes with spaces and returns a [`topcoat::view::Class`] value.

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

Entries can be strings or collections of entries, as shown above. Implement [`ClassViewParts`] to use your own type as an entry.

# Absent entries

`None`, empty strings, and entries whose condition is false are skipped without adding a space. When every entry is absent, the whole `class` attribute is omitted:

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

Use [`StaticClass`] to store a class list made only of string literals in a constant:

```rust
use topcoat::view::{StaticClass, class};

const BUTTON: StaticClass = class!("btn btn-lg rounded");
```

A class list's type depends on its entries. For other lists, let Rust infer the type with a `let` binding.

[`StaticClass`]: type.StaticClass.html
[`ClassViewParts`]: trait.ClassViewParts.html
[`class!`]: macro.class.html
[`topcoat::view::Class`]: struct.Class.html
