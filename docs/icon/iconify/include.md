Creates [`IconData`] constants from a staged [Iconify] set.

```rust,ignore
iconify::include!("feather");

view! {
    icon(data: feather::TARGET)
}
```

Unknown sets or icons cause a compile error with suggestions for similar names.

# Selections

The string argument selects what to include from a set:

- `include!("mdi")` expands to a module `mdi` with one `pub const` per icon.
- `include!("mdi:*")` expands to the same consts, inlined into the current scope.
- `include!("mdi:delete")` expands to the single const `DELETE`.

Add a visibility before the string, as in `include!(pub(crate) "mdi")`. It applies to the generated module, or to each constant when no module is generated.

Whole-set imports skip hidden icons and icons with rotation or flip properties. They allow unused constants. You can select a hidden icon by name, but selecting a rotated or flipped icon is an error.

# Names

Iconify's kebab-case icon names become `SCREAMING_SNAKE_CASE` consts: `trash-2` becomes `TRASH_2`, and a name with a leading digit gains a `_` prefix (`2fa` becomes `_2FA`). A set's aliases become consts of their own, next to the icons they point at.

Module names are the set's prefix in snake case (`simple-icons` becomes `simple_icons`); a prefix that turns into a keyword becomes a raw identifier (`box` becomes `r#box`).

# Staging

Stage the set from `build.rs` with [`BuildConfig`]. It downloads missing sets and reuses cached copies:

```rust,no_run
# #![allow(clippy::needless_doctest_main)]
// build.rs
fn main() {
    topcoat::icon::iconify::BuildConfig::new()
        .icon_set("mdi")
        .stage()
        .unwrap();
}
```

# Single icons

Use [`iconify_icon!`] when you need an icon expression, such as an argument to a component.

[Iconify]: https://iconify.design/
[`IconData`]: ../struct.IconData.html
[`BuildConfig`]: struct.BuildConfig.html
[`iconify_icon!`]: macro.iconify_icon.html
