Derives a builder that checks required properties at compile time.

For `ButtonProps`, the derive generates `ButtonPropsBuilder`. Its `build()` method is available only after every required property has been set.

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

Use field attributes to control the builder:

- `#[default]` uses [`Default::default()`] for an omitted property. `#[default(expr)]` uses a custom fallback instead. The fallback runs only when the property is omitted and does not require the type to implement [`Default`].
- `#[into]` lets the setter accept any value that converts to the field's type through `Into`. For example, callers can pass `&str` for a `String` field.

[`Default`]: https://doc.rust-lang.org/std/default/trait.Default.html
[`Default::default()`]: https://doc.rust-lang.org/std/default/trait.Default.html#tymethod.default
