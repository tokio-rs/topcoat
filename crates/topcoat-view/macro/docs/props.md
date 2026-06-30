Derives a typestate builder for a props struct.

For a struct `ButtonProps`, the derive generates a `ButtonPropsBuilder` whose `build()` method only becomes available once every required property has been set. Forgetting a property is a compile error, not a runtime panic.

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

Fields can be annotated with special attributes to modify the builder's behavior:

- `#[default]` makes a property optional. If it is not set, the field is filled with [`Default::default()`], so its type must implement [`Default`]. Use `#[default(expr)]` to supply a custom fallback instead, evaluated only when the property is not set; the type need not implement [`Default`] in that case.
- `#[into]` makes the generated setter accept any `impl Into<T>` instead of `T`.

[`Default`]: https://doc.rust-lang.org/std/default/trait.Default.html
[`Default::default()`]: https://doc.rust-lang.org/std/default/trait.Default.html#tymethod.default
