Declares a layout that wraps inner pages.

A layout wraps every page whose URL begins with the layout's URL. The layout's URL is the path string given to the attribute (`#[layout("/settings")]`). When no path is given, it is derived from the function's enclosing module path, kebab-cased, provided the function is reachable from a [`module_router!`](macro.module_router.html).

A layout registers like any other handler: pass the function name to [`RouterBuilder::layout`](struct.RouterBuilder.html#method.layout), or let [`discover`](trait.RouterBuilderDiscoverExt.html) or [`module_router!`](macro.module_router.html) collect it automatically.

# Handler signature

The function is `async` and returns a [`Result`](../type.Result.html) of a view. It takes the inner page's content as `slot`, of type [`Slot`](type.Slot.html), and interpolates it somewhere in its own view. It may also take [`cx: &Cx`](../context/struct.Cx.html). Both parameters are recognized by name, may appear in either order, and no other parameters are accepted.

A layout renders before the page it wraps resolves, so the page's error reaches the layout where the slot is interpolated. Emitting the slot from a [`live!`](../view/macro.live.html) region hands the layout that error instead of letting it become the response, which is how a branded error page is built; see the [error](../router/error/index.html) docs.

# Examples

Explicit path:

```rust
use topcoat::{Result, router::{Slot, layout}, view::{View, view}};

#[layout("/")]
async fn root_layout(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html>
            <body>
                <nav><a href="/">"Home"</a></nav>
                (slot)
            </body>
        </html>
    })
}
```

Module-derived path (in `src/app/settings.rs` under `module_router!()`, this wraps every page under `/settings`):

```rust
# use topcoat::{Result, router::{Slot, layout}, view::{View, view}};
#[layout]
async fn settings_layout(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        <section>
            <nav>"Settings nav"</nav>
            (slot)
        </section>
    })
}
```

# Nested layouts

When several layouts match a page, they nest from least specific (outermost) to most specific (innermost):

```rust
# use topcoat::{Result, router::{Slot, layout, page}, view::{View, view}};
#[layout("/")]
async fn root_layout(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! { <html><body>(slot)</body></html> })
}

#[layout("/settings")]
async fn settings_layout(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        <div class="settings-shell">
            <nav>"Settings nav"</nav>
            (slot)
        </div>
    })
}

#[page("/settings/profile")]
async fn profile() -> Result<impl View> {
    Ok(view! { <h1>"Profile"</h1> })
}
```

A request to `/settings/profile` renders `root_layout` > `settings_layout` > `profile`.

# Layouts as components

A layout doubles as a [component](../view/attr.component.html), taking a [`Slot`](type.Slot.html) as its `slot` property:

```rust
# use topcoat::{Result, router::{Slot, layout, page}, view::{View, view}};
# #[layout("/")]
# async fn root_layout(slot: Slot<'_>) -> Result<impl View> {
#     Ok(view! { <body>(slot)</body> })
# }
#[page("/standalone")]
async fn standalone() -> Result<impl View> {
    Ok(view! {
        root_layout(slot: Slot::new(view! { <p>"content"</p> }))
    })
}
```
