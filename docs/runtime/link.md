A link that opens a page without reloading the whole document. The server renders the destination and the runtime updates the current document with the result.

Use `link` inside a view, with [`href!`] pointing to a page in your app:

```rust
use topcoat::{
    Result,
    router::{href, page},
    runtime::link,
    view::{View, view},
};

#[page]
async fn home() -> Result<impl View> {
    Ok(view! {
        link(href: href!(about::page), "About")
    })
}

mod about {
    use super::*;

    #[page]
    pub async fn page() -> Result<impl View> {
        Ok(view! { <h1>"About"</h1> })
    }
}
# fn main() {}
```

With module routing, this links to `/about`. Enable `.runtime()` on the router, load the asset bundle, and include `topcoat::runtime::script()` in the document head. See the [runtime setup guide] for the full setup.

The component renders an ordinary `<a>`, so the link also works without JavaScript. Use a plain `<a href=(href!(about::page))>` when you want a normal page load.

# Prefetching

By default, a link starts loading its destination after a brief hover, on focus, or on touch. This is [`PrefetchMode::Intent`]. When the user follows the link, navigation can reuse that response even if it is still arriving.

Set `prefetch` on a link to choose when its destination loads:

```rust
use topcoat::{
    router::href,
    runtime::{PrefetchMode, link},
    view::view,
};
# #[topcoat::router::page("/products")]
# async fn products() -> topcoat::Result<impl topcoat::view::View> { Ok(view! { "Products" }) }
# #[topcoat::router::page("/reports")]
# async fn reports() -> topcoat::Result<impl topcoat::view::View> { Ok(view! { "Reports" }) }
# #[topcoat::view::component]
# async fn example() -> topcoat::Result<impl topcoat::view::View> {
Ok(view! {
    link(href: href!(products), prefetch: PrefetchMode::Viewport, "Products")
    link(href: href!(reports), prefetch: PrefetchMode::Never, "Reports")
})
# }
```

`Viewport` loads the destination when the link comes into view. `Never` waits until the user follows the link, while still navigating without a full reload. Prefetching is best effort; the runtime may skip it when the browser is set to save data or other prefetch requests are still loading.

Prefetching renders the destination on the server even if the user never opens it. Keep mutations in form submissions or procedures rather than page rendering.

Set the default for the app with [`RouterBuilderRuntimeExt::prefetch`](crate::RouterBuilderRuntimeExt::prefetch):

```rust
use topcoat::{
    router::Router,
    runtime::{PrefetchMode, RouterBuilderRuntimeExt},
};

let router = Router::builder()
    .runtime()
    .prefetch(PrefetchMode::Viewport)
    .build();
```

To choose a default for part of a page, add a `PrefetchMode` to its context:

```rust
use topcoat::{
    context::Cx,
    router::href,
    runtime::{PrefetchMode, link},
    view::{View, view},
};
# #[topcoat::router::page("/reports")]
# async fn reports() -> topcoat::Result<impl View> { Ok(view! { "Reports" }) }

fn reports_nav(cx: &Cx) -> impl View {
    let cx = cx.with(PrefetchMode::Never);
    view! { cx =>
        link(href: href!(reports), "Reports")
    }
}
```

A link's `prefetch` argument takes precedence over the request context, then the app context, then `Intent`. [`prefetch_mode`] returns the default for a context.

# Custom Anchors

Pass `attrs` to add classes and other HTML attributes:

```rust
# use topcoat::{router::href, runtime::link, view::{attributes, view}};
# #[topcoat::router::page("/products")]
# async fn products() -> topcoat::Result<impl topcoat::view::View> { Ok(view! { "Products" }) }
# #[topcoat::view::component]
# async fn example() -> topcoat::Result<impl topcoat::view::View> {
Ok(view! {
    link(
        href: href!(products),
        attrs: attributes! { class="nav-link" },
        "Products"
    )
})
# }
```

The `href` and `prefetch` arguments override matching attributes in `attrs`. The link's children can contain any view content.

For your own `<a>` markup, use [`link_attrs`] to create the destination and navigation attributes, then spread them onto the anchor:

```rust
use topcoat::{
    context::Cx,
    router::href,
    runtime::{link_attrs, prefetch_mode},
    view::{View, view},
};
# #[topcoat::router::page("/products")]
# async fn products() -> topcoat::Result<impl View> { Ok(view! { "Products" }) }

fn products_link(cx: &Cx) -> impl View {
    let attrs = link_attrs(cx, href!(products), prefetch_mode(cx));
    view! { cx =>
        <a class="product-card" (attrs)>
            <strong>"Browse products"</strong>
            <span>"See what's new"</span>
        </a>
    }
}
```

# Navigation Behavior

The current page stays visible until the destination's initial HTML arrives. The runtime updates the document, including its title, then applies any streamed content that follows. Signals declared on both pages keep their values. This lets shared components retain state across navigation.

Navigation updates the address bar and browser history. New pages scroll to the top, or to the target of a URL fragment. Back and forward navigation restore saved scroll positions. Links to fragments on the current page use the browser's normal behavior.

Modifier clicks, downloads, external links, and links targeting another window or frame also retain their normal browser behavior. If a destination needs scripts that have not been loaded, or its response cannot be used for runtime navigation, the browser loads the page normally.

[`href!`]: https://docs.rs/topcoat/latest/topcoat/router/macro.href.html
[runtime setup guide]: https://docs.rs/topcoat/latest/topcoat/runtime/index.html#setup
