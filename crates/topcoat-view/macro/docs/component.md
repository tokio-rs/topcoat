Use [`#[component]`][`component`] on an async function to define a reusable view. Its parameters become named properties, and it returns [`Result<impl View>`][`Result`].

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

Call a component inside [`view!`] with named parameters:

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

The component chooses where to render its children by inserting `(child)` in its template.

# Parameter attributes

A component's properties can be modified with attributes:

- `#[default]` makes the parameter optional; when not passed, it is set to `Default::default()`. Use `#[default(expr)]` to supply a custom fallback instead, evaluated only when the parameter is omitted. The type need not implement `Default` in that case.
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

Use `#[into]` when the parameter only needs to accept values convertible to one type.

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

When a component calls itself, box its returned view with [`boxed`](trait.ViewExt.html#method.boxed). This also works for indirect recursion, where components call each other.

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

Boxing the returned view of one component in each recursive cycle is enough.

[`Cx`]: ../context/struct.Cx.html
[`Result`]: ../type.Result.html
[`Child`]: struct.Child.html
[`View`]: trait.View.html
[`component`]: attr.component.html
[`view!`]: macro.view.html
