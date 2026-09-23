Expands a single icon from a staged [Iconify] icon set to an [`IconData`] expression that can be stored in a `const`.

```rust,ignore
use topcoat::icon::{IconData, iconify};

const DELETE: IconData = iconify::iconify_icon!("mdi:delete");
```

The argument is a `"set:icon"` reference. The icon name can also be one of the set's aliases. Because the macro expands to an expression, you can also use it directly in a view:

```rust,ignore
view! {
    icon(data: iconify::iconify_icon!("mdi:delete"))
}
```

The macro reads the staged set at compile time, so an unknown set or icon name is a compile error that suggests similar names. Icons that Iconify defines with a rotation or flip are not supported.

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

# Whole sets

To use many icons from a set, use [`include!`]. It expands a whole set, or a single icon, to named `const` items.

[Iconify]: https://iconify.design/
[`IconData`]: ../struct.IconData.html
[`BuildConfig`]: struct.BuildConfig.html
[`include!`]: macro.include.html
