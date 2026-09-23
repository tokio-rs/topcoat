Build HTML with [`view!`], using quoted text and parenthesized Rust expressions:

```rust
use topcoat::{
    Result,
    view::{View, component, view},
};

#[component]
async fn greeting(name: &str) -> Result<impl View> {
    Ok(view! {
        <h1>"Hello, " (name) "!"</h1>
    })
}
```

A view describes content to render. Use [`#[component]`][`component`] to give a reusable view named parameters, as `greeting` does above.

The [`view!`] guide explains template syntax. The [`component`] guide explains how to define and call components.

[`view!`]: macro.view.html
[`component`]: attr.component.html
