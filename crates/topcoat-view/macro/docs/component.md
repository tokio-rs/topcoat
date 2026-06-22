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

## Generics

Components can be generic; the function's generics carry over to the props struct. Because component futures must be `Send`, type parameters stored in props need a `Send` bound (and `Sync` when the view borrows them):

```rust,ignore
#[component]
async fn count<T: Send + Sync>(items: Vec<T>) -> Result {
    view! { <span>(items.len())</span> }
}
```

`impl Trait` parameters work too. Each occurrence is lifted into a generic type parameter on the props struct, keeping its bounds and adding `Send`:

```rust,ignore
#[component]
async fn shout(label: impl Into<String>) -> Result {
    view! { <b>(label.into().to_uppercase())</b> }
}
```

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

[`Cx`]: ../context/struct.Cx.html
[`Props`]: derive.Props.html
[`Result`]: ../type.Result.html
[`View`]: struct.View.html
[`component`]: attr.component.html
[`view!`]: macro.view.html
