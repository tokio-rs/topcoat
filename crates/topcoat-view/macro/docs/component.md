Components are async functions annotated with [`#[component]`][`component`]. They return a value implementing [`View`] through the usual Topcoat [`Result`] type, and can take typed parameters like any other Rust function.

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

Call components inside [`view!`] with a call syntax similar to function calls, but with named parameter syntax:

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

The name `key` is reserved: a `key:` argument keys the invocation's identity instead of setting a prop, so a component cannot declare a `key` parameter. See the keys section of the [`view!`] guide.

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

The trailing child nodes desugar to a `child` parameter whose value is a [`view! { ... }`][`view!`] containing those nodes.

# Parameter attributes

A component's properties can be modified with attributes:

- `#[default]` makes the parameter optional; when not passed, it is set to `Default::default()`. Use `#[default(expr)]` to supply a custom fallback instead, evaluated only when the parameter is omitted. The type need not implement `Default` in that case.
- `#[into]` lets callers pass anything that converts via `Into`. While you could use `impl Into<T>` instead, using `#[into]` calls `.into()` outside of your function body and prevents many monomorphizations of the function itself.

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

Prefer the `#[into]` attribute over `impl Into<T>` to reduce generic instantiations of your component body.

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

A component returns an anonymous view type, so a component calling itself, directly or indirectly, describes a type that contains itself. Break the cycle by erasing the view type of one component in it: box the view with [`boxed`](trait.ViewExt.html#method.boxed).

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

The other components in a cycle keep returning `impl View` as they are; one erased type is enough for all of them.

[`Cx`]: ../context/struct.Cx.html
[`Result`]: ../type.Result.html
[`Child`]: struct.Child.html
[`View`]: trait.View.html
[`component`]: attr.component.html
[`view!`]: macro.view.html
