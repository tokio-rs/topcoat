A component is a reusable piece of markup, written as an async function annotated with [`#[component]`][`component`]. It takes typed parameters like any other Rust function and returns a value implementing [`View`], wrapped in Topcoat's [`Result`] type.

```rust
use topcoat::{
    Result,
    view::{View, component, view},
};

#[component]
async fn badge(label: &str, tone: &str) -> Result<impl View> {
    Ok(view! {
        <span class=(format!("badge badge-{tone}"))>
            (label)
        </span>
    })
}
```

The function must be `async`, must declare a return type, and cannot take `self`. Each parameter must be a plain name, not a pattern like `(a, b)`. The macro replaces the function with a type of the same name that [`view!`] knows how to call. It also generates a props struct named after the function, such as `BadgeProps` for `badge`, with one field per parameter and a builder from the [`Props`] derive. Doc comments on the function are copied to both, so they show up when you hover a component call in your editor.

# Calling Components

Call a component inside [`view!`] like a function, but pass each argument by name:

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn badge(label: &str, tone: &str) -> Result<impl View> { Ok(view! { <span>(label)(tone)</span> }) }
# #[component]
# async fn example() -> Result<impl View> {
Ok(view! {
    <header>
        badge(
            label: "New",
            tone: "success",
        )
    </header>
})
# }
```

The arguments can come in any order. Leaving out a required argument is a compile error that names the missing parameter.

# Child Content

A component can accept child content through a parameter named `child` of type [`Child`]. Any view nodes after the named arguments in a call are collected into that parameter. Mark it `#[default]` so the component can also be called without children.

```rust
use topcoat::{
    Result,
    view::{Child, View, component, view},
};

#[component]
async fn panel(title: &str, #[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        <section class="panel">
            <h2>(title)</h2>
            <div class="panel-body">
                (child)
            </div>
        </section>
    })
}

# #[component]
# async fn badge(label: &str, tone: &str) -> Result<impl View> { Ok(view! { <span>(label)(tone)</span> }) }
# #[component]
# async fn example() -> Result<impl View> {
Ok(view! {
    panel(
        title: "Profile",
        // Child nodes:
        <p>"Account details"</p>
        badge(
            label: "Active",
            tone: "success",
        )
    )
})
# }
```

The child nodes are shorthand for a `child` argument whose value is a [`view!`] holding those nodes.

# Parameter Attributes

Two attributes change how a parameter is passed:

- `#[default]` makes the parameter optional. When a call leaves it out, it is set to `Default::default()`. Use `#[default(expr)]` to supply a different fallback, which is only evaluated when the parameter is left out. With `#[default(expr)]`, the type does not need to implement `Default`.
- `#[into]` lets callers pass any value that converts into the parameter type with `Into`, such as a `&str` for a `String` parameter. The conversion happens at the call site, outside of the component.

```rust
# use topcoat::{Result, view::{View, component, view}};
# #[derive(Default)]
# struct Tone;
#[component]
async fn badge(
    #[into] label: String,
    #[default] tone: Tone,
    #[default(80)] max_length: usize,
) -> Result<impl View> {
    // ...
#     Ok(view! { <span>(label)</span> })
}
```

# Generics

Components can be generic. Depending on how the type is used, you may need to require `Send` or `Sync`:

```rust
# use topcoat::{Result, view::{View, component, view}};
#[component]
async fn count<T: Send + Sync>(items: Vec<T>) -> Result<impl View> {
    Ok(view! { <span>(items.len())</span> })
}
```

`impl Trait` parameters work too:

```rust
# use topcoat::{Result, view::{View, component, view}};
#[component]
async fn shout(label: impl Into<String> + Send) -> Result<impl View> {
    Ok(view! { <b>(label.into().to_uppercase())</b> })
}
```

For conversions, prefer `#[into]` over `impl Into<T>`. A generic parameter makes the compiler generate a separate copy of the component body for every argument type, while `#[into]` converts at the call site and keeps a single copy.

# Request Context

A component can access the current request context by declaring a parameter named `cx` of type [`&Cx`][`Cx`]. It is filled in automatically and is not passed at the call site.

```rust
use topcoat::{
    Result,
    context::Cx,
    router::request::uri,
    view::{View, component, view},
};

#[component]
async fn current_path(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        <span>(uri(cx).path())</span>
    })
}
```

# Recursive Components

A component returns an anonymous view type. When a component calls itself, directly or through other components, that type would have to contain itself, which Rust does not allow. Break the cycle by erasing the view type of one component in it with [`boxed`](trait.ViewExt.html#method.boxed):

```rust
use topcoat::{
    Result,
    view::{View, ViewExt, component, view},
};

#[component]
async fn countdown(n: u32) -> Result<impl View> {
    Ok(view! {
        <li>(n)</li>
        if n > 0 {
            countdown(n: n - 1)
        }
    }
    .boxed())
}
```

The other components in the cycle can keep returning `impl View`. One erased type is enough to break the cycle.

[`Cx`]: ../context/struct.Cx.html
[`Result`]: ../type.Result.html
[`Child`]: struct.Child.html
[`Props`]: derive.Props.html
[`View`]: trait.View.html
[`component`]: attr.component.html
[`view!`]: macro.view.html
