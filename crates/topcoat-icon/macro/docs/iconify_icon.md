Creates an [`IconData`] expression for one icon from a staged [Iconify] set. The result can be assigned to a constant.

```rust,ignore
use topcoat::icon::IconData;

const DELETE: IconData = iconify::iconify_icon!("mdi:delete");
```

Pass a `"set:icon"` reference. The icon name can be an alias. You can also use the macro directly in a view:

```rust,ignore
view! {
    icon(data: iconify::iconify_icon!("mdi:delete"))
}
```

Unknown sets or icons cause a compile error with suggestions for similar names. Icons with rotation or flip properties are not supported.

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

# Whole sets

Use [`include!`] to generate named constants for an icon set.

[Iconify]: https://iconify.design/
[`IconData`]: ../struct.IconData.html
[`BuildConfig`]: struct.BuildConfig.html
[`include!`]: macro.include.html
