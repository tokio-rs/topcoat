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

- `#[default]` makes a property optional. If it is not set, the field is filled with [`Default::default()`]. The field's type must implement [`Default`].
- `#[into]` makes the generated setter accept `impl Into<T>` instead of `T`, so `.label("Save")` works for a `String` field.

# Typestate

The builder tracks each required property in a type parameter that flips to [`Set`] when the property's setter is called. `build()` requires every marker to implement [`IsSet`], so this fails to compile:

```rust,compile_fail
# use topcoat::view::Props;
# enum ButtonKind { Primary }
# #[derive(Props)]
# struct ButtonProps {
#     #[into]
#     label: String,
#     kind: ButtonKind,
#     #[default]
#     disabled: bool,
# }
// error: missing required property `kind`
let props = ButtonProps::builder().label("Save").build();
```

Setters can be called more than once; later calls replace the earlier value.

# Generics

Generic structs are supported. The struct's generics, bounds, and `where` clauses carry over to the builder:

```rust
# use topcoat::view::Props;
#[derive(Props)]
struct ListProps<T: Clone> {
    items: Vec<T>,
    #[default]
    compact: bool,
}
```

# The `Props` Trait

The derive also implements the [`Props`] trait, whose associated `Builder` type names the builder in its initial state. The generated `builder()` function is available both as an inherent function and through the trait.

[`Default`]: https://doc.rust-lang.org/std/default/trait.Default.html
[`Default::default()`]: https://doc.rust-lang.org/std/default/trait.Default.html#tymethod.default
[`IsSet`]: trait.IsSet.html
[`Props`]: trait.Props.html
[`Set`]: struct.Set.html
