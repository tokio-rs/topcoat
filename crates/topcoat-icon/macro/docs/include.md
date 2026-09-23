Expands icons from a staged [Iconify] icon set to [`IconData`] constants.

```rust,ignore
use topcoat::icon::{icon, iconify};

iconify::include!("feather");

view! {
    icon(data: feather::TARGET)
}
```

[Iconify] collects many open source icon sets in one format. The macro reads the staged set at compile time, so an unknown set or icon name is a compile error that suggests similar names.

# Selections

The string argument selects what to include from a set:

- `include!("mdi")` expands to a module `mdi` with one `pub const` per icon.
- `include!("mdi:*")` expands to the same constants, directly in the current scope.
- `include!("mdi:delete")` expands to the single constant `DELETE`.

You can put a visibility before the string, like `include!(pub(crate) "mdi")`. In the first form it applies to the module, and in the other two to each constant.

The whole-set forms skip icons that the set marks as hidden, which usually means deprecated. They also skip icons with a rotation or flip. Their constants allow `dead_code`, so including a whole set does not warn about icons you do not use. Naming a hidden icon directly still works, but naming an icon with a rotation or flip is a compile error.

# Names

Iconify's kebab-case icon names become `SCREAMING_SNAKE_CASE` constants: `trash-2` becomes `TRASH_2`. A name that starts with a digit gets a `_` prefix, so `2fa` becomes `_2FA`. Each alias in a set becomes a constant of its own, next to the icon it points to.

The module name is the set's prefix in snake case, so `simple-icons` becomes `simple_icons`. A prefix that is a Rust keyword becomes a raw identifier, so `box` becomes `r#box`.

# Staging

A build script stages the icon sets with [`BuildConfig`]. It downloads each set from Iconify, or reads it from a local cache directory:

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

This macro expands to items. To use an icon as an expression, directly in a view or under a name you choose, use [`iconify_icon!`]. It takes a single `"set:icon"` reference and expands to an [`IconData`] expression.

[Iconify]: https://iconify.design/
[`IconData`]: ../struct.IconData.html
[`BuildConfig`]: struct.BuildConfig.html
[`iconify_icon!`]: macro.iconify_icon.html
