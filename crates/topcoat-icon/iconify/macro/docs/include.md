Expands to `const` [`IconData`] icons from a staged [Iconify] icon set.

```rust,ignore
iconify::include!("feather");

view! {
    icon(data: feather::TARGET)
}
```

Iconify aggregates the icons of over 150 open source icon sets. The staged set is read at compile time, so every reference is checked while you build: an unknown set or icon name is a compile error with near-miss suggestions.

# Selections

The string argument selects what to include from a set:

- `include!("mdi")` expands to a module `mdi` with one `pub const` per icon.
- `include!("mdi:*")` expands to the same consts, inlined into the current scope.
- `include!("mdi:delete")` expands to the single const `DELETE`.

An optional leading visibility applies to the expansion, e.g. `include!(pub(crate) "mdi")`: to the module in the first form, and to each const in the other two.

The whole-set forms skip icons their set marks as hidden, which usually means deprecated, and icons that carry a rotation or flip. Their consts also allow `dead_code`, so including a whole set does not warn about unused icons. Naming a hidden icon explicitly still works; naming a rotated or flipped one is an error.

# Names

Iconify's kebab-case icon names become `SCREAMING_SNAKE_CASE` consts: `trash-2` becomes `TRASH_2`, and a name with a leading digit gains a `_` prefix (`2fa` becomes `_2FA`). A set's aliases become consts of their own, next to the icons they point at.

Module names are the set's prefix in snake case (`simple-icons` becomes `simple_icons`); a prefix that turns into a keyword becomes a raw identifier (`box` becomes `r#box`).

# Staging

Sets are staged by the crate's build script through [`BuildConfig`], which downloads them from Iconify or picks them up from a local cache directory:

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

This macro expands to items. To use an icon as an expression instead, inline in a view or behind a name of your choosing, reach for [`iconify_icon!`], which takes a single `"set:icon"` reference and expands to a const-evaluable [`IconData`] expression.

[Iconify]: https://iconify.design/
[`IconData`]: ../struct.IconData.html
[`BuildConfig`]: struct.BuildConfig.html
[`iconify_icon!`]: macro.iconify_icon.html
