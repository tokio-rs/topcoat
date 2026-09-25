XML sitemaps for Topcoat routes.

A [sitemap](https://www.sitemaps.org) lists URLs for crawlers to discover. Enable the `sitemap` feature and return a [`Sitemap`] from a route to serve an XML sitemap.

# Serving a sitemap

This example serves a sitemap at `/sitemap.xml`. Add one entry with [`url`](Sitemap::url) or an iterator of entries with [`urls`](Sitemap::urls). Each entry can be a path string or a [`SitemapUrl`] with optional metadata.

```rust
use topcoat::{
    Result,
    router::{
        content::sitemap::{ChangeFrequency, Sitemap, SitemapUrl},
        route,
    },
};

#[route(GET "/sitemap.xml")]
async fn sitemap() -> Result<Sitemap> {
    let posts = ["first-post", "second-post"];
    Ok(Sitemap::new()
        .url("/")
        .url(SitemapUrl::new("/about").change_frequency(ChangeFrequency::Monthly))
        .urls(posts.map(|slug| format!("/posts/{slug}"))))
}
```

The sitemap format requires absolute URLs, so register the base URL the application is publicly reachable at on the router. An entry given as a root-relative path is resolved against it when the response is rendered; an entry that is already an absolute `http` or `https` URL is used as is. Rendering a relative entry without a registered base URL panics.

```rust,no_run
use topcoat::router::Router;

let router = Router::builder().base_url("https://example.com").build();
```

# Entry fields

Beyond its location, a [`SitemapUrl`] carries the optional metadata of the sitemap format. Every builder method replaces the field it sets.

- [`last_modified`](SitemapUrl::last_modified) records when the page last changed. It accepts a value convertible into `SystemTime`.
- [`change_frequency`](SitemapUrl::change_frequency) hints how often crawlers should revisit the page, from [`Always`](ChangeFrequency::Always) for a page that changes on every access to [`Never`](ChangeFrequency::Never) for an archived one.
- [`priority`](SitemapUrl::priority) ranks the page relative to the other pages of the site, from `0.0` to `1.0`; crawlers treat an entry without a priority as `0.5`.

```rust
use std::time::SystemTime;

use topcoat::router::content::sitemap::{ChangeFrequency, SitemapUrl};

let url = SitemapUrl::new("/posts/42")
    .last_modified(SystemTime::now())
    .change_frequency(ChangeFrequency::Weekly)
    .priority(0.8);
```

# The path under `module_router!`

A module-derived path cannot contain a dot, because module names are converted to kebab-case. To serve the sitemap from a module tree, declare a `sitemap` module and override its segment with `segment!`; a rename is used as written.

```rust
// src/app/sitemap.rs: serves /sitemap.xml
use topcoat::{Result, router::{content::sitemap::Sitemap, route}};

topcoat::router::segment!(rename = "sitemap.xml");

#[route(GET)]
async fn sitemap() -> Result<Sitemap> {
    Ok(Sitemap::new().url("/"))
}
```

Registering the explicit-path route from the first example instead works the same under `module_router!`; pass it to the builder by name or let `discover` collect it.
