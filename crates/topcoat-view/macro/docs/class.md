The [`class!`] macro builds a [`topcoat::view::Class`] value: a space-separated list of HTML classes assembled from individual entries.

Use it in the value position of a `class` attribute when the list mixes static and conditional parts:

```rust
# #[topcoat::view::component]
# async fn example() -> topcoat::Result {
use topcoat::view::{class, view};

let is_active = true;

view! {
    <button class=(class!("btn", "btn-lg", "active" if is_active))>"Save"</button>
}
# }
```

# Syntax

The body is a comma-separated list of entries. Each entry is a Rust expression, optionally followed by a trailing condition:

- `expr` includes the entry unconditionally.
- `expr if cond` includes the entry only when `cond` is true.
- `expr if cond else alt` includes `expr` when `cond` is true and `alt` otherwise.

```rust
# #[topcoat::view::component]
# async fn example() -> topcoat::Result {
use topcoat::view::{class, view};

let variant: Option<&str> = Some("primary");
let sizes = vec!["px-4".to_owned(), "py-2".to_owned()];
let enabled = false;

view! {
    <button class=(class!(
        "btn",
        variant,
        sizes,
        "cursor-pointer" if enabled else "opacity-50",
    ))>"Save"</button>
}
# }
```

An entry can be any value implementing [`ClassViewParts`]: string types, `Option`s of them, a `Vec` or array of entries, or another [`Class`]. Implement the trait for your own types to use them as entries.

# Absent entries

An absent entry is skipped without leaving a leftover space: `None`, empty strings, and entries whose condition is false contribute neither text nor a separator. When every entry is absent, the surrounding element omits the whole `class` attribute:

```rust
# #[topcoat::view::component]
# async fn example() -> topcoat::Result {
use topcoat::view::{class, view};

let variant: Option<&str> = None;

// Renders `<p></p>`.
view! {
    <p class=(class!(variant, "active" if false))></p>
}
# }
```

[`Class`]: struct.Class.html
[`ClassViewParts`]: trait.ClassViewParts.html
[`class!`]: macro.class.html
[`topcoat::view::Class`]: struct.Class.html
