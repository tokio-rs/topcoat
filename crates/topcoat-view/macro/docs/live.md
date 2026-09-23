A page normally renders in full before the browser sees any of it, so a single slow database query or upstream request delays everything, even the parts that are ready. The [`live!`] and [`emit!`] macros let a page send what it has right away and stream in the slow parts when they finish. The updates travel over the same response, without any client-side fetching.

[`live!`] marks a region of the page whose content can still change while the response streams. Its body is ordinary async Rust. Inside the body, [`emit!`] renders markup into the region, and each emission replaces the previous one in the browser.

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

The heading and the loading message reach the browser immediately. While `fetch_quote` runs, the rest of the page streams as usual. Once the quote is ready, it replaces the loading message in place. The browser needs no client library for this, because the response carries everything the swap requires.

The page waits for a region's first emission and renders it as part of the document. So start the body with something that is ready right away, like the loading message above.

The two most common uses come as ready-made components: [`suspense`] shows a fallback while one piece of content loads, and [`error_boundary`] catches a failed render. Both are described at the end of this guide. If one of them fits, you do not need the macros at all.

# Emitting Many Times

[`emit!`] accepts everything [`view!`] does: elements, text, expressions, control flow, and components. Between emissions, the body is plain async Rust, so it can await work, loop, and branch. Since each emission replaces the previous one, a live region can report the progress of a long-running task as it happens:

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

Emissions can also run concurrently, for example as `async` blocks under `join!`. Every one of them reaches the browser, but since they all replace the same region, the last one to arrive stays visible. Emit concurrently when only the final emission matters, and emit in sequence when each one should be seen.

# The Emit Token

A live region has to emit at least once, so that it never leaves a hole in the page. The body's return type reminds you of this. [`emit!`] evaluates to a [`Result`] holding an [`EmitToken`], and the body must return one, so the natural way to finish is with an emission, as in the examples above. Earlier emissions use `?` to stop when one fails.

When the body does not end with an emission, create the token yourself:

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

This region emits inside the loop, so returning `Ok(EmitToken)` afterwards only satisfies the type. The token is only a compile-time reminder. The body still has to emit at least once.

# Handling Errors

An emission fails when its markup fails to render, for example when a component it calls returns an error. The error comes back as the `Err` value of [`emit!`] instead of ending the stream. The body decides what happens next: it can pass the error on with `?`, or handle it and emit a fallback instead.

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

An error returned from the body propagates like any other rendering error. If it happens after the page started streaming, the response can no longer change its status code. The [router's error guide] describes how errors and redirects behave in a streaming response.

# A Region Is A View

A live region is a view like any other. The examples above insert one into a [`view!`] body, but a component can also return one directly, or take one as child content:

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

Several regions on one page stream independently, and each replaces its own content when it is ready. Emitted markup can itself contain components and other live regions.

A region's id comes from its source location and the identity of the enclosing context, so it stays the same across renders. When a `view!` loop contains live regions, put a `#[key(...)]` attribute on the loop to tell its iterations apart, as described in the [`view!`] guide.

# Request Context

Inside a [`component`], `#[page]`, or `#[layout]`, the request context is in scope implicitly, and emitted markup can call components without extra setup. In a plain function, name the context at the start of the live region, the same way as with [`view!`]:

```rust
use topcoat::{context::Cx, view::{View, emit, live}};

fn greeting(cx: &Cx) -> impl View {
    live! { cx => emit! { <p>"Hello"</p> } }
}
```

With `cx =>`, the returned view holds its own clone of the context. Without it, the view borrows the context of the enclosing body. Emissions use the region's context, and a single emission can use a different one with `emit! { cx => ... }`. See the [`view!`] guide's section on rendering outside a component.

# Suspense And Error Boundaries

The [`suspense`] component shows a fallback until its child content is ready. The child gets the first chance to render: when it is ready right away, it renders in place and the fallback never shows. Otherwise, the fallback goes out with the page and the child replaces it once it is ready:

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

The [`error_boundary`] component renders its child content, and replaces it with a fallback built from the error when any part of it fails:

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

Both are small components built on the same regions as [`live!`]. Use the macros directly when a region needs more than they offer, like progress updates or retrying after a failure.

[`EmitToken`]: struct.EmitToken.html
[`Result`]: ../type.Result.html
[`component`]: attr.component.html
[`emit!`]: macro.emit.html
[`error_boundary`]: struct.error_boundary.html
[`live!`]: macro.live.html
[`suspense`]: struct.suspense.html
[`view!`]: macro.view.html
[router's error guide]: ../router/error/index.html
