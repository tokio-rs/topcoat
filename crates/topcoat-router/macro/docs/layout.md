Declares a layout that wraps inner pages.

A layout wraps pages whose registered path starts with the layout's path, compared segment by segment. Set an absolute path with `#[layout("/settings")]`. Under [`module_router!`](macro.module_router.html), omit the path to derive it from the enclosing module, or use `./` to extend the module path. For example, `#[layout("./admin")]` in `src/app/settings.rs` wraps pages under `/settings/admin`.

A layout registers like any other handler: pass the function name to [`RouterBuilder::layout`](struct.RouterBuilder.html#method.layout), or let [`discover`](trait.RouterBuilderDiscoverExt.html) or [`module_router!`](macro.module_router.html) collect it automatically.

# Handler signature

The function must be `async` and return a [`Result`](../type.Result.html) containing a [`View`](../view/trait.View.html). It receives the inner content as `slot: Slot<'_>` and may also take [`cx: &Cx`](../context/struct.Cx.html). The macro recognizes these parameters by name. They may appear in either order, and no other parameters are accepted.

Render `slot` where the inner content should appear. Wrap it in an [`error_boundary`](../view/struct.error_boundary.html) to show a custom view if it fails. See the [error guide](../router/error/index.html).

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

Path below the module (in `src/app/settings.rs` under `module_router!()`, this wraps every page under `/settings/admin`):

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
