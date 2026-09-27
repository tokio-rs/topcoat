The [`#[component]`][`component`] attribute defines an async function that returns a view. Components accept typed parameters and return [`Result<impl View>`][`Result`].

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

# Calling Components

Call components inside [`view!`] with named arguments:

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

# Child Content

If a component accepts a parameter named `child` with type [`Child`], any extra view nodes in the call are collected and passed as that child view. Give it `#[default]` so the component can also be called without children.

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

You can also pass a view explicitly as the `child` argument.

# Parameter attributes

A parameter can have these attributes:

- `#[default]` uses `Default::default()` when the caller omits the argument. `#[default(expr)]` uses a custom fallback instead. The fallback runs only when the argument is omitted and does not require the type to implement `Default`.
- `#[into]` accepts any value that converts to the parameter's type through `Into`. The component receives the converted value.

```rust
# use topcoat::{Result, view::{View, component, view}};
# #[derive(Default)]
# struct Tone;
#[component]
async fn badge(#[into] label: String, #[default] tone: Tone, #[default(80)] max_length: usize) -> Result<impl View> {
    // ...
#     Ok(view! { <span>(label)</span> })
}
```

# Generics

Components can be generic. Depending on usage, you may need to declare the type as `Send` or `Sync`:

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

Prefer `#[into]` when the component only needs the converted value.

# Request Context

Components can ask for the current request context by declaring a `cx` parameter that borrows [`Cx`]:

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

A recursive component needs to box its view with [`boxed`](trait.ViewExt.html#method.boxed). This gives the view a type with a known size:

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

For mutually recursive components, boxing one view in the cycle is enough.

[`Cx`]: ../context/struct.Cx.html
[`Result`]: ../type.Result.html
[`Child`]: struct.Child.html
[`View`]: trait.View.html
[`component`]: attr.component.html
[`view!`]: macro.view.html
