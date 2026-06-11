# The [`component`] macro

Components are async functions annotated with [`#[component]`][`component`]. They return a [`View`] through the usual Topcoat [`Result`] type, and can take typed parameters like any other Rust function.

```rust,ignore
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

Call components inside [`view!`] with function-call syntax. Named arguments use `name: value`:

```rust,ignore
view! {
    <header>
        badge(
            label: "New",
            tone: "success",
        )
    </header>
}
```

All component parameters are named parameters, except `child`, which can be passed unnamed in the last position. After the named arguments, unnamed child nodes are written like normal [`view!`] content; multiple child nodes do not need commas between them.

## Child Content

If a component accepts a parameter named `child` with type [`View`], any extra view nodes in the call are collected and passed as that child view.

```rust,ignore
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

Conceptually, those trailing child nodes are the same thing as a `child` parameter whose value is a [`view! { ... }`][`view!`] containing those nodes.

## Props

The macro turns the function's parameters (except `cx`) into a generated props struct named after the component in PascalCase plus `Props` (`badge` becomes `BadgeProps`), which derives [`Props`] to get a typestate builder. Component calls in [`view!`] go through that builder, so leaving out a parameter is a compile error naming the missing property.

Parameters can use the same attributes as [`Props`] fields:

- `#[default]` makes the parameter optional; when not passed, it gets `Default::default()`.
- `#[into]` lets callers pass anything that converts via `Into`.

```rust,ignore
#[component]
async fn badge(#[into] label: String, #[default] tone: Tone) -> Result {
    // ...
}
```

A `child` parameter is always optional and defaults to an empty [`View`].

## Request Context

Components can ask for the current request context by declaring a `cx` parameter that borrows [`Cx`]:

```rust,ignore
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

[`Cx`]: https://docs.rs/topcoat/latest/topcoat/context/struct.Cx.html
[`Props`]: https://docs.rs/topcoat/latest/topcoat/view/derive.Props.html
[`Result`]: https://docs.rs/topcoat/latest/topcoat/type.Result.html
[`View`]: https://docs.rs/topcoat/latest/topcoat/view/struct.View.html
[`component`]: https://docs.rs/topcoat/latest/topcoat/view/attr.component.html
[`view!`]: https://docs.rs/topcoat/latest/topcoat/view/macro.view.html
