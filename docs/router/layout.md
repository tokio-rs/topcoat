Declares a layout that wraps inner pages.

Use `#[layout]` without a path string with [module routing](macro.module_router.html). The enclosing module determines the URL prefix. A path starting with `./` extends the module path. An absolute path, such as `#[layout("/settings")]`, chooses the URL independently of the module tree and requires separate registration.

[`module_router!`](macro.module_router.html) registers module-derived handlers. For explicit paths, pass the function name to [`RouterBuilder::layout`](struct.RouterBuilder.html#method.layout) or use [`discover`](trait.RouterBuilderDiscoverExt.html).

# Handler signature

The function must be `async` and take [`cx: &Cx`](../context/struct.Cx.html) as its first parameter, even when unused. Its second parameter must be `slot: Slot<'_>`, which receives the inner content. No other parameters are accepted. The function must return a [`Result`](../type.Result.html) containing a [`View`](../view/trait.View.html).

Render `slot` where the inner content should appear. Wrap it in an [`error_boundary`](../view/struct.error_boundary.html) to show a custom view if it fails. See the [error guide](../router/error/index.html).

# Route context

Attach metadata with `with(...)`, such as `#[layout(with(Title("My shop")))]` or `#[layout("/", with(Title("My shop")))]`. You can add multiple `with(...)` entries, separated by commas. The values are initialized when the router is built and added to the context of every page the layout wraps, before any layer, layout, or handler runs. Inner layouts and pages can shadow these defaults. See [route context](../context/index.html#route-context) for reading and combining metadata.

# Examples

Module-derived path (in `src/app/settings.rs` under `module_router!()`, this wraps every page under `/settings`):

```rust
# use topcoat::{Result, context::Cx, router::{Slot, layout}, view::{View, view}};
#[layout]
async fn settings_layout(cx: &Cx, slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        <section>
            <nav>"Settings nav"</nav>
            (slot)
        </section>
    })
}
```

Explicit path:

```rust
use topcoat::{Result, context::Cx, router::{Slot, href, layout}, view::{View, view}};
# #[topcoat::router::page("/")]
# async fn home(cx: &Cx) -> Result<impl View> { Ok(topcoat::view::view! { "Home" }) }

#[layout("/")]
async fn root_layout(cx: &Cx, slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html>
            <body>
                <nav><a href=(href!(home))>"Home"</a></nav>
                (slot)
            </body>
        </html>
    })
}
```

Path below the module (in `src/app/settings.rs` under `module_router!()`, this wraps every page under `/settings/admin`):

```rust
# use topcoat::{Result, context::Cx, router::{Slot, layout}, view::{View, view}};
#[layout("./admin")]
async fn admin_layout(cx: &Cx, slot: Slot<'_>) -> Result<impl View> {
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
# use topcoat::{Result, context::Cx, router::{Slot, layout, page}, view::{View, view}};
#[layout]
async fn root_layout(cx: &Cx, slot: Slot<'_>) -> Result<impl View> {
    Ok(view! { <html><body>(slot)</body></html> })
}

mod settings {
    use super::*;

    #[layout]
    async fn settings_layout(cx: &Cx, slot: Slot<'_>) -> Result<impl View> {
        Ok(view! {
            <div class="settings-shell">
                <nav>"Settings nav"</nav>
                (slot)
            </div>
        })
    }
    
    mod profile {
        use super::*;
    
        #[page]
        async fn profile(cx: &Cx) -> Result<impl View> {
            Ok(view! { <h1>"Profile"</h1> })
        }
    }
}
# fn main() {}
```

A request to `/settings/profile` renders `root_layout` > `settings_layout` > `profile`.

# Layouts as components

A layout doubles as a [component](../view/attr.component.html), taking a [`Slot`](type.Slot.html) as its `slot` property:

```rust
# use topcoat::{Result, context::Cx, router::{Slot, layout, page}, view::{View, view}};
# #[layout]
# async fn root_layout(cx: &Cx, slot: Slot<'_>) -> Result<impl View> {
#     Ok(view! { <body>(slot)</body> })
# }
#[page]
async fn standalone(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        root_layout(slot: Slot::new(view! { <p>"content"</p> }))
    })
}
```
