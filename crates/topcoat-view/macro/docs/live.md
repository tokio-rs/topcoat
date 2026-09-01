A page normally renders in full before the browser sees any of it, so a single slow database query or upstream request delays everything, even the parts that are ready. The [`live!`] and [`emit!`] macros let a page send what it has right away and stream the slow parts in when they finish, over the same response and without any client-side fetching.

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

The heading and the loading message reach the browser immediately. While `fetch_quote` runs, the rest of the page streams as usual, and once the quote is ready it replaces the loading message in place. The browser needs no client library for this; the response carries everything the swap requires.

The page waits for a region's first emission and renders it with the rest of the document, so start the body with something that is ready right away, like the loading message above.

The two most common shapes, a fallback that waits for one piece of content and a guard that catches a failed render, come prepackaged as the [`suspense`] and [`error_boundary`] components, described at the end of this guide. If one of them fits, you never need the macros; they are the general form behind both.

# Emitting More Than Once

[`emit!`] accepts everything [`view!`] does: elements, text, interpolated expressions, control flow, and components. Between emissions the body is plain async Rust, so it can await work, loop, and branch. Because each emission replaces the previous one, a live region can narrate a long-running task as it happens:

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

# The Emit Token

A live region has to emit at least once, so it never leaves a hole in the page. The body's return type is a safety net that reminds you of this: [`emit!`] evaluates to a [`Result`] carrying an [`EmitToken`], and the body returns one, so the natural way to finish is to end with an emission, as the examples above do. Intermediate emissions use a `?` to stop when one fails.

When the body's control flow does not end with an emission, construct the token yourself to opt out of the reminder:

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

The region here emits inside the loop, so returning `Ok(EmitToken)` afterwards only satisfies the type. The token is a compile-time reminder, nothing more; the body still has to emit at least once.

# Handling Errors

An emission fails when the markup inside it fails to render, for example when a component it calls returns an error. The failure comes back as the `Err` value of [`emit!`] instead of ending the stream, and the body decides what happens next: propagate it with `?`, or handle it and emit a fallback in its place.

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

A live region is a view like any other. The examples above interpolate one into a [`view!`] body, but a component can just as well return one directly, or take one as child content:

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

# Request Context

Inside a [`component`], `#[page]`, or `#[layout]`, the request context is in scope implicitly and emitted markup can call components with no ceremony. In a plain function, name the context at the start of the emission, the same way [`view!`] does: `emit! { cx => ... }`. See the [`view!`] guide's section on rendering outside a component.

# Suspense And Error Boundaries

The [`suspense`] component is a live region that shows a fallback until its child content is ready:

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

Both are small components built on [`live!`] and [`emit!`]. Reach for the macros directly when a region needs more than they cover, like progress updates or retrying after a failure.

[`EmitToken`]: struct.EmitToken.html
[`Result`]: ../type.Result.html
[`component`]: attr.component.html
[`emit!`]: macro.emit.html
[`error_boundary`]: fn.error_boundary.html
[`live!`]: macro.live.html
[`suspense`]: fn.suspense.html
[`view!`]: macro.view.html
[router's error guide]: https://docs.rs/topcoat/latest/topcoat/router/error/index.html
