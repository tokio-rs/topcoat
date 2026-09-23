Use [`live!`] to send initial content and replace it while the response streams. This lets you show a loading message while waiting for data.

The body is async Rust. Each [`emit!`] renders markup that replaces the region's previous content in the browser:

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn example() -> Result<impl View> {
Ok(view! {
    <h1>"Quote of the day"</h1>
    (live! {
        emit! { <p>"Loading..."</p> }?;
        let quote = fetch_quote().await;
        emit! { <blockquote>(quote)</blockquote> }
    })
})
# }
# async fn fetch_quote() -> &'static str { "..." }
```

The page first shows the heading and loading message. When `fetch_quote` finishes, the quote replaces the message. The response includes what the browser needs to update the region.

The page waits for a region's first emission and renders it with the rest of the document, so start the body with something that is ready right away, like the loading message above.

For a loading fallback, you can use [`suspense`] instead. To show a fallback when rendering fails, use [`error_boundary`]. Both are described below.

# Emitting Many Times

[`emit!`] accepts the same syntax as [`view!`]. Between emissions, await work or use Rust control flow. For example, report progress as a task runs:

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn example() -> Result<impl View> {
Ok(view! {
    <h1>"Progress"</h1>
    (live! {
        for percent in 0..100 {
            emit! { <p>"Working... " (percent) "%"</p> }?;
            run_step().await;
        }
        emit! { <p>"Done!"</p> }
    })
})
# }
# async fn run_step() {}
```

Emissions can run concurrently, for example in `async` blocks under `join!`. Each replaces the same region, so the last to arrive stays visible. Emit in sequence when their order matters.

# The Emit Token

A live region must emit at least once. Its body returns [`Result<EmitToken>`][`Result`], which is also the result of [`emit!`]. End the body with an emission, and use `?` on earlier emissions to propagate failures.

If control flow does not end with an emission, return `Ok(EmitToken)`:

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn example() -> Result<impl View> {
Ok(view! {
    (live! {
        while let Some(price) = next_price().await {
            emit! { <p>"BTC: " (price)</p> }?;
        }
        Ok(EmitToken)
    })
})
# }
# async fn next_price() -> Option<u32> { None }
```

Constructing the token does not emit content. In this example, `next_price()` must yield at least one value so the loop emits at least once.

# Handling Errors

If markup fails to render, [`emit!`] returns the error. Propagate it with `?`, or handle it and emit a fallback:

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn forecast() -> Result<impl View> { Ok(view! { <p>"Sunny"</p> }) }
# #[component]
# async fn example() -> Result<impl View> {
Ok(view! {
    <h1>"Weather"</h1>
    (live! {
        emit! { <p>"Loading..."</p> }?;
        match emit! { forecast() } {
            Err(error) => emit! {
                <p>"The forecast is unavailable: " (error.to_string())</p>
            },
            emitted => emitted,
        }
    })
})
# }
```

An error the body returns propagates like any other rendering error. When it happens after the page started streaming, the response can no longer change its status code; the [router's error guide] describes how errors and redirects behave in a streaming response.

# A Region Is A View

A live region implements `View`. A component can return one directly:

```rust
use topcoat::{
    Result,
    view::{View, component, emit, live, view},
};

#[component]
async fn daily_quote() -> Result<impl View> {
    Ok(live! {
        emit! { <p>"Loading..."</p> }?;
        let quote = fetch_quote().await;
        emit! { <blockquote>(quote)</blockquote> }
    })
}
# async fn fetch_quote() -> &'static str { "..." }

#[component]
async fn page() -> Result<impl View> {
    Ok(view! {
        <h1>"Quote of the day"</h1>
        daily_quote()
    })
}
```

Several regions on one page stream independently, each replacing its own content as it becomes ready, and emitted markup can itself contain components and further live regions.

When a `view!` loop contains live regions, give each item a stable key with `#[key(expr)]`, as described in the [`view!`] guide. This keeps each region associated with its item across renders.

# Request Context

Inside a [`component`], the request context is available implicitly. In a plain function, pass it at the start of the live region:

```rust
use topcoat::{context::Cx, view::{View, emit, live}};

fn greeting(cx: &Cx) -> impl View {
    live! { cx => emit! { <p>"Hello"</p> } }
}
```

With `cx =>`, the returned view owns a clone of the supplied context. Without `cx =>`, it borrows the enclosing body's context. Emissions use that context implicitly. An individual emission can use another context with `emit! { cx => ... }`. See the [`view!`] guide's section on rendering outside a component.

# Suspense And Error Boundaries

The [`suspense`] component shows a fallback until its child content is ready. The child gets the first chance to render: when it is ready right away, it renders in place and the fallback never shows. Otherwise the fallback goes out with the page and the child replaces it once it resolves:

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn quote() -> Result<impl View> { Ok(view! { <blockquote>"..."</blockquote> }) }
# #[component]
# async fn example() -> Result<impl View> {
Ok(view! {
    suspense(
        fallback: view! { <p>"Loading..."</p> },
        quote()
    )
})
# }
```

The [`error_boundary`] component renders its child content and swaps in a fallback built from the error when any part of it fails:

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn stats() -> Result<impl View> { Ok(view! { <p>"3 visits today"</p> }) }
# #[component]
# async fn example() -> Result<impl View> {
Ok(view! {
    error_boundary(
        fallback: |error| Ok(view! {
            <p>"The stats are unavailable: " (error.to_string())</p>
        }),
        stats()
    )
})
# }
```

Use [`live!`] directly when you need to control when content is replaced, such as for progress updates.

[`EmitToken`]: struct.EmitToken.html
[`Result`]: ../type.Result.html
[`component`]: attr.component.html
[`emit!`]: macro.emit.html
[`error_boundary`]: struct.error_boundary.html
[`live!`]: macro.live.html
[`suspense`]: struct.suspense.html
[`view!`]: macro.view.html
[router's error guide]: https://docs.rs/topcoat/latest/topcoat/router/error/index.html
