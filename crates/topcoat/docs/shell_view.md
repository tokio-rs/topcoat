`ShellView` streams a page shell first, then replaces placeholders as deferred views finish. It is useful when a page has a fast container and slower independent sections. Return it from a [`route`](crate::router::route) container; pages, layouts, and components continue to return ordinary views and can be rendered inside that container.

Enable the `shell-view` feature, then build the response around ordinary [`View`](crate::view::View) values:

```rust
use topcoat::{
    Result,
    context::Cx,
    shell_view::ShellView,
    router::route,
    view::view,
};

# async fn load_activity() -> Result<&'static str> { Ok("Signed in") }
#[route(GET "/")]
async fn home(cx: &Cx) -> Result<ShellView> {
    let mut page = ShellView::builder(cx);
    let activity = page.defer(
        view! { <p aria-busy="true">"Loading activity..."</p> }?,
        |cx| async move {
            let message = load_activity().await?;
            let cx = cx.as_ref();
            view! { cx => <p>(message)</p> }
        },
    );

    let shell = view! {
        <!DOCTYPE html>
        <html>
            <body>
                <h1>"Dashboard"</h1>
                (activity)
            </body>
        </html>
    }?;
    Ok(page.finish(shell))
}
```

[`defer`](ShellViewBuilder::defer) returns a normal `View` containing the placeholder. Insert it anywhere a view expression is accepted. The render closure receives an owned [`CxHandle`](crate::context::CxHandle), so request context stays available after the handler returns. Bind `handle.as_ref()` to an identifier when using the explicit `cx =>` form of `view!` inside the closure.

The shell is the first response chunk. Deferred futures start when the response body is polled, run concurrently, and emit replacement scripts in completion order. A slow fragment does not delay a faster one.

# Components

A deferred closure can render components through a nested `view!` call:

```rust
# use topcoat::{Result, context::Cx, shell_view::ShellView, view::{component, view}};
# #[component]
# async fn recommendations() -> Result { view! { <p>"Recommendations"</p> } }
# async fn example(cx: &Cx) -> Result<ShellView> {
let mut page = ShellView::builder(cx);
let recommendations_slot = page.defer(
    view! { <p>"Finding recommendations..."</p> }?,
    |cx| async move {
        let cx = cx.as_ref();
        view! { cx => recommendations() }
    },
);

let shell = view! { cx => <section>(recommendations_slot)</section> }?;
Ok(page.finish(shell))
# }
```

# Composition

Shell views compose at their container boundary. Pass a child to [`include`](ShellViewBuilder::include), insert the returned shell into the parent, then finish the parent. The parent adopts the child's deferred work, so all fragments use one response stream.

```rust
# use topcoat::{Result, context::Cx, shell_view::ShellView, view::view};
# async fn sidebar(cx: &Cx) -> Result<ShellView> { Ok(ShellView::from_view(view! { cx => <aside></aside> }?)) }
# async fn example(cx: &Cx) -> Result<ShellView> {
let child = sidebar(cx).await?;
let mut page = ShellView::builder(cx);
let child = page.include(child);
let shell = view! { cx => <main>(child)</main> }?;
Ok(page.finish(shell))
# }
```

# Response behavior

Status codes and headers declared by the shell are applied before streaming begins. Declarations in deferred views cannot change headers that were already sent. Each completed fragment is delivered by a small inline script, so the page's Content Security Policy must allow those scripts.
