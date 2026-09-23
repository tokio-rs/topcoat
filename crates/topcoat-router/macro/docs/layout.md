Declares a layout: a view that wraps the pages below its path.

A layout wraps every page whose path starts with the layout's path. The attribute takes an optional path string. With an absolute path, such as `#[layout("/settings")]`, the layout uses that path. Without a path, the layout uses the path of its module, as described in [`module_router!`](macro.module_router.html). A path that starts with `./` is added to the end of the module path, so `#[layout("./admin")]` in `src/app/settings.rs` wraps the pages under `/settings/admin`.

Register a layout with an absolute path by passing its name to [`RouterBuilder::layout`](struct.RouterBuilder.html#method.layout), or let [`discover`](trait.RouterBuilderDiscoverExt.html) collect it. A layout with a module-derived path is registered by [`module_router!`](macro.module_router.html).

# Handler signature

The function must be `async` and return a [`Result`](../type.Result.html) of a value that implements [`View`](../view/trait.View.html). It must take the wrapped content as `slot: Slot<'_>` (see [`Slot`](type.Slot.html)) and place it somewhere in its view. It can also take the request context as [`cx: &Cx`](../context/struct.Cx.html). Both parameters are recognized by name and can come in either order. No other parameters are allowed.

The layout decides where and when the wrapped page, or the next nested layout, is rendered. A layout can also catch errors from the pages it wraps by putting the slot in an [`error_boundary`](../view/struct.error_boundary.html). This is how you build a custom error page. See the [error guide](error/index.html).

# Examples

An absolute path:

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

A module-derived path. In `src/app/settings.rs` under `module_router!()`, this wraps every page under `/settings`:

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

A path below the module. In `src/app/settings.rs` under `module_router!()`, this wraps every page under `/settings/admin`:

```rust
# use topcoat::{Result, router::{Slot, layout}, view::{View, view}};
#[layout("./admin")]
async fn admin_layout(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        <section>
            <nav>"Admin nav"</nav>
            (slot)
        </section>
    })
}
```

# Nested layouts

When several layouts match a page, they nest by path length. The layout with the shortest path is the outermost, and the one with the longest path is the innermost:

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

A request to `/settings/profile` renders `profile` inside `settings_layout`, inside `root_layout`.

# Layouts as components

A layout is also a [component](../view/attr.component.html). It takes a [`Slot`](type.Slot.html) as its `slot` prop:

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
