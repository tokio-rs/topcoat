# The `component` macro

Components are async functions annotated with `#[component]`. They return a `View` through the usual Topcoat `Result` type, and can take typed parameters like any other Rust function.

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

## Calling Components

Call components inside `view!` with function-call syntax. Named arguments use `name: value`:

```rust
view! {
    <header>
        badge(
            label: "New",
            tone: "success",
        )
    </header>
}
```

All component parameters are named parameters, except `child`, which can be passed unnamed in the last position. After the named arguments, unnamed child nodes are written like normal `view!` content; multiple child nodes do not need commas between them.

## Child Content

If a component accepts a parameter named `child` with type `View`, any extra view nodes in the call are collected and passed as that child view.

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

view! {
    panel(
        title: "Profile",
        <p>"Account details"</p>
        badge(
            label: "Active",
            tone: "success",
        )
    )
}
```

Conceptually, those trailing child nodes are the same thing as a `child` parameter whose value is a `view! { ... }` containing those nodes.

## Request Context

Components can ask for the current request context by declaring a parameter named `cx: &Cx`:

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
