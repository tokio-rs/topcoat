[Inertia.js](https://inertiajs.com) lets a client framework render page components while Topcoat remains responsible for routing, data loading, redirects, and validation. This integration implements the Inertia.js v3 page and visit protocol. It does not support the legacy `data-page` mount attribute used by older adapters.

Everything below is re-exported from `topcoat::inertia` and gated behind the `inertia` feature. Asset-based versioning also requires `asset`.

```toml
# Cargo.toml
[dependencies]
topcoat = { version = "0.5.0", features = ["inertia", "asset"] }
```

# Configure the router

An Inertia application needs a root HTML callback. The callback inserts [`inertia_root`] where the client application mounts. The helper renders an inert `application/json` script followed by an empty mount element. Both use the configured root ID, and JSON containing `</script>` is escaped safely.

```rust
use topcoat::{
    context::Cx,
    inertia::{Page, inertia_root},
    view::{HtmlContext, NodeViewParts, PartsWriter, View, ViewParts},
};

fn root(cx: &Cx, page: &Page) -> View {
    let mut parts = ViewParts::new();
    let mut html = PartsWriter::new(&mut parts, HtmlContext::Text);
    html.push_str_unescaped("<!DOCTYPE html><html><head><title>My app</title></head><body>");
    inertia_root(page).into_view_parts(cx, &mut html);
    html.push_str_unescaped("<script type=\"module\" src=\"/app.js\"></script></body></html>");
    View::new(parts)
}
```

Install the Inertia layer before `.cookies()`. The default [`CookieFlashStore`] uses Topcoat's private cookie jar, so it requires a persistent [`Key`](crate::cookie::Key) in app context. Generate that key once during provisioning and load the same value in every process or serverless isolate. Calling `Key::generate()` at application startup would invalidate cookies after every restart.

```rust,no_run
# use topcoat::{context::Cx, inertia::{InertiaConfig, Page, RouterBuilderInertiaExt}, cookie::{Key, RouterBuilderCookieExt}, router::Router, view::View};
# fn root(_: &Cx, _: &Page) -> View { View::empty() }
# fn persistent_cookie_key() -> Key { Key::generate() }
let router = Router::builder()
    .app_context(persistent_cookie_key())
    .inertia(InertiaConfig::new(root))
    .cookies()
    .build();
```

The default cookie is secure. For local development over plain HTTP only, configure `CookieFlashStore::new().secure(false)`. Keep the secure default in production.

# Render pages

Return an [`Inertia`] page from a route handler. The builder owns plain values and accepts futures that borrow `cx`, so a prop is loaded only if the current visit selects it.

```rust
use topcoat::{
    Result,
    context::Cx,
    inertia::{Inertia, InertiaResponse},
};

async fn load_permissions(_: &Cx) -> Result<Vec<&'static str>> {
    Ok(vec!["read"])
}

async fn users(cx: &Cx) -> Result<InertiaResponse> {
    Inertia::new("Users/Index")
        .prop("title", "Users")
        .lazy("permissions", load_permissions(cx))
        .render(cx)
        .await
}
```

An ordinary browser request runs the root callback. An Inertia request receives JSON with `X-Inertia: true`, `Content-Type: application/json`, and `Vary: X-Inertia`. Unrelated API, asset, and streaming responses are left unchanged.

See the [props guide](https://docs.rs/topcoat-inertia/latest/topcoat_inertia/#props) for partial reloads, deferred loading, merging, once props, rescue behavior, nested paths, sharing, and infinite scroll.

# Shared props

Use [`InertiaConfig::share_with`] for values needed by many pages. The callback runs during rendering and may add a future that borrows the request context.

```rust
# use topcoat::{Result, context::Cx, inertia::{InertiaConfig, Page}, view::View};
# async fn load_auth(_: &Cx) -> Result<&'static str> { Ok("Ada") }
# fn root(_: &Cx, _: &Page) -> View { View::empty() }
let config = InertiaConfig::new(root).share_with(|cx, props| {
    props.lazy("auth", load_auth(cx));
    props.always("locale", "en");
    Ok(())
});
```

Configured shared props run first, request-local [`share`] props run next, and page props run last. Later declarations win. The reserved `errors` prop is always an object and is automatically listed as shared. Do not declare `errors` through a normal prop or sharing API.

# Redirects and asset versions

Use [`inertia_location`] when a response must leave the Inertia application. It returns a normal redirect for ordinary requests and the v3 location response for Inertia requests.

Set a stable asset version with [`InertiaConfig::version`]. With the `asset` feature, [`InertiaConfig::version_from_assets`] derives the version from the content-hashed names in an [`AssetCatalog`](crate::asset::AssetCatalog). A stale Inertia `GET` becomes a full location visit before a page body is used.

The layer also applies v3 redirect rules: 301 and 302 responses after `PUT`, `PATCH`, or `DELETE` become 303; fragment redirects use `X-Inertia-Redirect`; and optional external redirect conversion uses `X-Inertia-Location`. Speculative prefetches keep normal fragment redirects, and 307 or 308 redirects always keep HTTP method-preserving semantics.

# Flash data and validation

Use [`flash`] and [`flash_errors`] before a redirect. The target page receives page flash outside history props and validation errors through `props.errors`. Flash is consumed only after it is included in a rendered page. This means a prefetch can consume flash if it renders the target page; avoid prefetching redirect destinations whose one-time messages must be shown to the user.

The private cookie store has a deliberate size limit. Use a custom [`FlashStore`] backed by your session or database when error payloads can be large or when browser cookies are unsuitable. See the [flash](https://docs.rs/topcoat-inertia/latest/topcoat_inertia/#flash-storage) and [validation](https://docs.rs/topcoat-inertia/latest/topcoat_inertia/#validation) guides for the full lifecycle.

Topcoat sessions verify mutation origins, but they do not replace CSRF-aware application design. Install sessions for authenticated mutations, keep cookies same-site, and use ordinary `POST`, `PUT`, `PATCH`, or `DELETE` routes for writes.

# Production errors

Prop futures return `topcoat::Result`, so application failures follow normal Topcoat error handling. A failed deferred prop can opt into [`Prop::rescue`], which omits that prop and reports its path in `rescuedProps`. Other resolver, serialization, configuration, and storage errors fail the response. Install the same production error handling and logging used by the rest of the application, and do not expose internal error strings to clients.
