The [`class!`] macro builds a [`Class`] value: a list of HTML classes, separated by spaces, put together from individual entries.

Use it as the value of a `class` attribute when the list mixes fixed and conditional classes:

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

The body is a comma-separated list of entries. Each entry is a Rust expression, optionally followed by a condition:

- `expr` always includes the entry.
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

An entry can be any value implementing [`ClassViewParts`]. This includes strings, `Option`s of them, another [`Class`], and an [`AttributeValue`] taken from an [`Attributes`] collection. Implement the trait for your own types to use them as entries. An entry without a condition can also be a `Vec` or an array of entries, which adds each of its items.

# Absent Entries

Absent entries are skipped without leaving an extra space. `None`, empty strings, and entries whose condition is false add neither text nor a separator. When every entry is absent, the element leaves out the `class` attribute entirely:

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

# Static Class Lists

`class!` is also useful for a class list that never changes. The macro escapes string literals at compile time, so the list renders faster than the same classes stored in a `&'static str` constant:

```rust
use topcoat::view::{StaticClass, class};

const BUTTON: StaticClass = class!("btn btn-lg rounded");
```

The type of a `class!` value depends on its entries. A list of only string literals always has the same type, named [`StaticClass`], so it can be stored in a constant. For any other list, the type is best left to inference in a `let` binding.

[`Attributes`]: struct.Attributes.html
[`AttributeValue`]: enum.AttributeValue.html
[`Class`]: struct.Class.html
[`ClassViewParts`]: trait.ClassViewParts.html
[`StaticClass`]: type.StaticClass.html
[`class!`]: macro.class.html
