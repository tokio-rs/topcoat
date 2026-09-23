The [`live!`] and [`emit!`] macros stream updates to part of a page. Use them to show content while slower work finishes. Updates travel over the page's HTTP response.

[`live!`] marks a region of the page whose content can still change while the response streams. Its body is ordinary async Rust. Inside the body, [`emit!`] renders markup into the region, and every emission replaces the previous one in the browser.

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

The heading and loading message render before `fetch_quote` finishes. The quote then replaces the loading message. The response includes the script needed to update the region.

The page waits for a region's first emission and renders it with the rest of the document, so start the body with something that is ready right away, like the loading message above.

Use [`suspense`] to show a fallback while content loads, or [`error_boundary`] to show one when rendering fails. The last section covers these components.

# Emitting Many Times

[`emit!`] accepts the same syntax as [`view!`]. The surrounding body is ordinary async Rust. Emit repeatedly to show the progress of a task:

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

Emissions can run concurrently, for example in `async` blocks under `join!`. Each replaces the same region, so the last to arrive remains visible. Emit sequentially when their order matters.

# The Emit Token

A live region must emit at least once. Its body returns [`Result<EmitToken>`][`Result`], the same type as [`emit!`], so it can end with an emission. Use `?` on intermediate emissions to stop if rendering fails.

If the body cannot end with an emission, return the token explicitly:

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

Returning `Ok(EmitToken)` satisfies the return type but does not emit content. The body still needs to emit at least once, so the loop above must receive a price.

# Handling Errors

If markup fails to render, [`emit!`] returns `Err`. Propagate the error with `?`, or handle it and emit a fallback:

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

A live region implements `View`. A component can return it directly:

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

Regions on the same page stream independently. Emitted markup can also contain live regions.

Regions need a stable identity across renders. When a `view!` loop contains live regions, use `#[key(item)]` to distinguish its iterations. See the [`view!`] guide.

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

The [`suspense`] component shows a fallback while its child content loads. If the child is ready immediately, the fallback never appears:

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

Use [`live!`] directly when you need to control the updates, such as showing progress or retrying after a failure.

[`EmitToken`]: struct.EmitToken.html
[`Result`]: ../type.Result.html
[`component`]: attr.component.html
[`emit!`]: macro.emit.html
[`error_boundary`]: struct.error_boundary.html
[`live!`]: macro.live.html
[`suspense`]: struct.suspense.html
[`view!`]: macro.view.html
[router's error guide]: https://docs.rs/topcoat/latest/topcoat/router/error/index.html
