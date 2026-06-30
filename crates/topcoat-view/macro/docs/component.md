Components are async functions annotated with [`#[component]`][`component`]. They return a [`View`] through the usual Topcoat [`Result`] type, and can take typed parameters like any other Rust function.

```rust
use topcoat::{
    Result,
    view::{component, view},
};

#[component]
async fn badge(label: &str, tone: &str) -> Result {
    view! {
        <span class=(format!("badge badge-{tone}"))>
            (label)
        </span>
    }
}
```

# Calling Components

Call components inside [`view!`] with a call syntax similar to function calls, but with named parameter syntax:

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn badge(label: &str, tone: &str) -> Result { view! { <span>(label)(tone)</span> } }
# #[component]
# async fn example() -> Result {
view! {
    <header>
        badge(
            label: "New",
            tone: "success",
        )
    </header>
}
# }
```

# Child Content

If a component accepts a parameter named `child` with type [`View`], any extra view nodes in the call are collected and passed as that child view.

```rust
use topcoat::{
    Result,
    view::{View, component, view},
};

#[component]
async fn panel(title: &str, child: View) -> Result {
    view! {
        <section class="panel">
            <h2>(title)</h2>
            <div class="panel-body">
                (child)
            </div>
        </section>
    }
}

# #[component]
# async fn badge(label: &str, tone: &str) -> Result { view! { <span>(label)(tone)</span> } }
# #[component]
# async fn example() -> Result {
view! {
    panel(
        title: "Profile",
        // Child nodes:
        <p>"Account details"</p>
        badge(
            label: "Active",
            tone: "success",
        )
    )
}
# }
```

The trailing child nodes desugar to a `child` parameter whose value is a [`view! { ... }`][`view!`] containing those nodes.

# Parameter attributes

A component's properties can be modified with attributes:

- `#[default]` makes the parameter optional; when not passed, it is set to `Default::default()`. Use `#[default(expr)]` to supply a custom fallback instead, evaluated only when the parameter is omitted. The type need not implement `Default` in that case.
- `#[into]` lets callers pass anything that converts via `Into`.

```rust
# use topcoat::{Result, view::{component, view}};
# #[derive(Default)]
# struct Tone;
#[component]
async fn badge(#[into] label: String, #[default] tone: Tone, #[default(80)] max_length: usize) -> Result {
    // ...
#     view! { <span>(label)</span> }
}
```

# Generics

Components can be generic. Depending on usage, you may need to declare the type as `Send` or `Sync`:

```rust
# use topcoat::{Result, view::{component, view}};
#[component]
async fn count<T: Send + Sync>(items: Vec<T>) -> Result {
    view! { <span>(items.len())</span> }
}
```

`impl Trait` parameters work too:

```rust
# use topcoat::{Result, view::{component, view}};
#[component]
async fn shout(label: impl Into<String> + Send) -> Result {
    view! { <b>(label.into().to_uppercase())</b> }
}
```

Prefer the `#[into]` attribute over `impl Into<T>` to reduce generic instantiations of your component body.

# Request Context

Components can ask for the current request context by declaring a `cx` parameter that borrows [`Cx`]:

```rust
use topcoat::{
    Result,
    context::Cx,
    router::uri,
    view::{component, view},
};

#[component]
async fn current_path(cx: &Cx) -> Result {
    view! {
        <span>(uri(cx).path())</span>
    }
}
```

[`Cx`]: ../context/struct.Cx.html
[`Result`]: ../type.Result.html
[`View`]: struct.View.html
[`component`]: attr.component.html
[`view!`]: macro.view.html
