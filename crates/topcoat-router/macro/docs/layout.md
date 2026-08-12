Declares a layout that wraps inner pages.

A layout wraps every page whose URL begins with the layout's URL. The layout's URL is the path string given to the attribute (`#[layout("/settings")]`). When no path is given, it is derived from the function's enclosing module path, kebab-cased, provided the function is reachable from a [`module_router!`](macro.module_router.html).

A layout registers like any other handler: pass the function name to [`RouterBuilder::layout`](struct.RouterBuilder.html#method.layout), or let [`discover`](trait.RouterBuilderDiscoverExt.html) or [`module_router!`](macro.module_router.html) collect it automatically.

# Handler signature

The function is `async` and returns [`Result`](../type.Result.html). It takes the inner page as `slot`, of type [`View`](../view/struct.View.html), and places it with `(slot?)` somewhere in its own view, where the `?` propagates the inner render's error. It may also take [`cx: &Cx`](../context/struct.Cx.html). Both parameters are recognized by name, may appear in either order, and no other parameters are accepted.

A layout sees the inner page's error before it becomes a response: `slot.take_error()` claims it, so the layout can match on the error instead of propagating it; the [router](index.html#status-codes-and-headers) docs show a branded not-found page built this way.

# Examples

Explicit path:

```rust
use topcoat::{Result, router::layout, view::{View, view}};

#[layout("/")]
async fn root_layout(slot: View) -> Result {
    view! {
        <!DOCTYPE html>
        <html>
            <body>
                <nav><a href="/">"Home"</a></nav>
                (slot?)
            </body>
        </html>
    }
}
```

Module-derived path (in `src/app/settings.rs` under `module_router!()`, this wraps every page under `/settings`):

```rust
# use topcoat::{Result, router::layout, view::{View, view}};
#[layout]
async fn settings_layout(slot: View) -> Result {
    view! {
        <section>
            <nav>"Settings nav"</nav>
            (slot?)
        </section>
    }
}
```

# Nested layouts

When several layouts match a page, they nest from least specific (outermost) to most specific (innermost):

```rust
# use topcoat::{Result, router::{layout, page}, view::{View, view}};
#[layout("/")]
async fn root_layout(slot: View) -> Result {
    view! { <html><body>(slot?)</body></html> }
}

#[layout("/settings")]
async fn settings_layout(slot: View) -> Result {
    view! {
        <div class="settings-shell">
            <nav>"Settings nav"</nav>
            (slot?)
        </div>
    }
}

#[page("/settings/profile")]
async fn profile() -> Result {
    view! { <h1>"Profile"</h1> }
}
```

A request to `/settings/profile` renders `root_layout` > `settings_layout` > `profile`.

# Layouts as components

A layout doubles as a [component](../view/attr.component.html): its child content feeds the `slot` parameter:

```rust
# use topcoat::{Result, router::{layout, page}, view::{View, view}};
# #[layout("/")]
# async fn root_layout(slot: View) -> Result {
#     view! { <body>(slot?)</body> }
# }
#[page("/standalone")]
async fn standalone() -> Result {
    view! {
        root_layout(
            <p>"content"</p>
        )
    }
}
```
