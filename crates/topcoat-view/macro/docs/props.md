Derives a builder for a props struct that checks at compile time that every required property is set.

For a struct `ButtonProps`, the derive generates a `ButtonPropsBuilder` and a `ButtonProps::builder()` function that returns it with nothing set. The builder has one setter method per field. Its `build()` method only becomes available once every required property has been set, so forgetting one is a compile error instead of a runtime panic. The derive also implements the [`Props`](trait.Props.html) trait for the struct.

The [`component`](attr.component.html) macro uses this derive for the props struct it generates, so you rarely need to write it yourself.

```rust
# use topcoat::view::Props;
# enum ButtonKind { Primary }
#[derive(Props)]
struct ButtonProps {
    #[into]
    label: String,
    kind: ButtonKind,
    #[default]
    disabled: bool,
}

let props = ButtonProps::builder()
    .label("Save")
    .kind(ButtonKind::Primary)
    .build();
```

The derive only supports structs with named fields. A field cannot be named `build`, because its setter would clash with the `build()` method. Doc comments on a field are copied to its setter.

# Field Attributes

Two attributes change how a field is set:

- `#[default]` makes a property optional. If it is not set, the field is filled with [`Default::default()`], so its type must implement [`Default`]. Use `#[default(expr)]` to supply a different fallback, which is only evaluated when the property is not set. With `#[default(expr)]`, the type does not need to implement [`Default`].
- `#[into]` makes the setter accept any `impl Into<T>` instead of `T`. Callers can then pass convertible values, like a `&str` for a `String` field, and the setter converts them.

[`Default`]: https://doc.rust-lang.org/std/default/trait.Default.html
[`Default::default()`]: https://doc.rust-lang.org/std/default/trait.Default.html#tymethod.default
