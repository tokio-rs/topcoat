Derives a builder that checks required properties at compile time.

For a struct `ButtonProps`, the derive generates a `ButtonPropsBuilder`. Call `ButtonProps::builder()`, set the properties, then call `build()`. The call to `build()` only compiles when every required property has been set.

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

# Field Attributes

Use these field attributes to control the builder:

- `#[default]` makes a property optional. If it is not set, the field is filled with [`Default::default()`], so its type must implement [`Default`]. Use `#[default(expr)]` to supply a custom fallback instead, evaluated only when the property is not set; the type need not implement [`Default`] in that case.
- `#[into]` makes the generated setter accept any `impl Into<T>` instead of `T`, so callers can pass convertible values like a `&str` for a `String` field and the setter performs the conversion.

[`Default`]: https://doc.rust-lang.org/std/default/trait.Default.html
[`Default::default()`]: https://doc.rust-lang.org/std/default/trait.Default.html#tymethod.default
