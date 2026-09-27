XML sitemaps for Topcoat routes.

The handler examples use [module routing](https://docs.rs/topcoat/latest/topcoat/router/macro.module_router.html), the recommended default. File comments show where each handler belongs under an `app` module that calls `module_router!()`.

A [sitemap](https://www.sitemaps.org) lists URLs for crawlers to discover. Enable the `sitemap` feature and return a [`Sitemap`] from a route to serve an XML sitemap.

# Serving a sitemap

This example uses [module routing](crate::module_router) to serve a sitemap at `/sitemap.xml`. The `segment!` rename preserves the dot, which a Rust module name cannot contain. Add one entry with [`url`](Sitemap::url) or an iterator of entries with [`urls`](Sitemap::urls). Each entry can be a path string or a [`SitemapUrl`] with optional metadata.

```rust
// src/app/sitemap_xml.rs
use topcoat::{
    Result,
    router::{
        content::sitemap::{ChangeFrequency, Sitemap, SitemapUrl},
        route,
    },
};

topcoat::router::segment!(rename = "sitemap.xml");

#[route(GET)]
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
use topcoat::router::module_router;
use topcoat::router::Router;

let router = module_router!().base_url("https://example.com").build();
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
