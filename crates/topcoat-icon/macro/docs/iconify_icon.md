Returns one icon from a staged [Iconify] set as an [`IconData`] value. The result can be stored in a constant.

```rust,ignore
use topcoat::icon::IconData;

const DELETE: IconData = iconify::iconify_icon!("mdi:delete");
```

The argument is a `"set:icon"` reference; the icon name may also be one of the set's aliases. Because the expansion is an expression, it can sit right inside a view:

```rust,ignore
view! {
    icon(data: iconify::iconify_icon!("mdi:delete"))
}
```

The staged set is read at compile time, so the reference is checked while you build: an unknown set or icon name is a compile error with near-miss suggestions. Icons that carry a rotation or flip are not supported.

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

# Whole sets

Use [`include!`] to declare named constants for a set's icons.

[Iconify]: https://iconify.design/
[`IconData`]: ../struct.IconData.html
[`BuildConfig`]: struct.BuildConfig.html
[`include!`]: macro.include.html
