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

Choose the app's default with [`RouterBuilderRuntimeExt::prefetch`](crate::RouterBuilderRuntimeExt::prefetch):

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

For a different default in part of a page, give that part a child context containing a `PrefetchMode`:

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

A link uses its `prefetch` argument when one is given. Otherwise, it uses the `PrefetchMode` in its context, or `Intent` if there is none. A value added with `Cx::with` overrides the router's default for that child context. Use [`prefetch_mode`] to read the context's default.

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

# View Transitions

Set `vt` to run the navigation inside a [View Transition], giving the page a typed name your stylesheet can target:

```rust
# use topcoat::{router::href, runtime::link, view::view};
# #[topcoat::router::page("/products")]
# async fn products() -> topcoat::Result<impl topcoat::view::View> { Ok(view! { "Products" }) }
# #[topcoat::view::component]
# async fn example() -> topcoat::Result<impl topcoat::view::View> {
Ok(view! {
    link(href: href!(products), vt: "detail", "Products")
})
# }
```

Separate multiple types with a space. The browser animates from the old page to the new one, and `:active-view-transition-type()` selectors and `::view-transition-*` rules match while it runs:

```css
:root:active-view-transition-type(detail)::view-transition-old(root) {
    animation: fade-out 0.2s ease both;
}
```

Transitions need browser support. The runtime skips them where the API is missing or the visitor prefers reduced motion, and the navigation still happens either way. It also falls back to an untyped transition on browsers that support transitions but not transition types, so the page still animates, and only your typed keyframes are skipped.

# Shared Elements

By default the whole page cross-fades. To morph an element into its counterpart on the destination, mark the element with [`VT_SHARED_ATTRIBUTE`] and give the destination's element the same [`view-transition-name`][vt-name] in its style:

```rust
# use topcoat::{router::href, runtime::link, view::view};
# #[topcoat::router::page("/products")]
# async fn products() -> topcoat::Result<impl topcoat::view::View> { Ok(view! { "Products" }) }
# #[topcoat::view::component]
# async fn example() -> topcoat::Result<impl topcoat::view::View> {
# let id = 7;
Ok(view! {
    link(
        href: href!(products),
        vt: "detail",
        <img
            data-topcoat-vt-shared=(format!("product-{id}"))
            src="/products/shoes.jpg"
            alt="Shoes"
        />
    )
})
# }
```

The marker's value becomes the element's `view-transition-name` when the link is followed, pairing it with the destination element that carries the same name. The runtime writes the name before the browser snapshots the current page and cleans up its own mark after the swap, so you never write the timing or clean it up. After the swap, the destination's markup determines the styles the element keeps. The marker is inert on its own, and it may go on any element, not only an image.

A link may mark more than one element, to morph several at once. Each needs its **own** name: a `view-transition-name` must be unique among the elements rendered when the transition starts, so marking two elements with the same name makes the browser skip the animation. Pair each origin with one destination, and give every pair its own name.

The destination's matching element carries the same name in its style:

```rust
# use topcoat::{context::Cx, view::view};
# use topcoat::view::View;
# #[topcoat::view::component]
# async fn hero(cx: &Cx) -> topcoat::Result<impl View> {
# let id = 7;
Ok(view! {
    <img
        src="/products/shoes.jpg"
        alt="Shoes"
        style=(format!("view-transition-name: product-{id}"))
    />
})
# }
```

A name must be unique among the elements rendered when the transition starts, because two elements sharing one makes the browser skip the animation. Derive it from whatever identifies the item and the element, so a page showing the same item twice does not repeat it.

[view-transition-name]: https://developer.mozilla.org/docs/Web/CSS/view-transition-name

For your own `<a>` markup, use [`VT_ATTRIBUTE`] with `attrs`.

# Navigation Behavior

The current page stays visible until the destination's initial HTML arrives. The runtime updates the document, including its title, then applies any streamed content that follows. Signals declared on both pages keep their values. This lets shared components retain state across navigation.

Navigation updates the address bar and browser history. New pages scroll to the top, or to the target of a URL fragment. Back and forward navigation restore saved scroll positions. Links to fragments on the current page use the browser's normal behavior.

Modifier clicks, downloads, external links, and links targeting another window or frame also retain their normal browser behavior. If a destination needs scripts that have not been loaded, or its response cannot be used for runtime navigation, the browser loads the page normally.

[`href!`]: https://docs.rs/topcoat/latest/topcoat/router/macro.href.html
[View Transition]: https://developer.mozilla.org/docs/Web/API/View_Transition
[vt-name]: https://developer.mozilla.org/docs/Web/CSS/view-transition-name
[runtime setup guide]: https://docs.rs/topcoat/latest/topcoat/runtime/index.html#setup
