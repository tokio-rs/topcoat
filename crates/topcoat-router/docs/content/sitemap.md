XML sitemaps for topcoat routes.

A [sitemap](https://www.sitemaps.org) lists the URLs of a site so crawlers can discover every page, along with optional metadata about each one. This module (behind the `sitemap` feature) provides the [`Sitemap`] response: a route builds one entry by entry and returns it, and the response is sent as the sitemap XML document with `Content-Type: application/xml`.

# Serving a sitemap

Crawlers expect the sitemap at `/sitemap.xml`. Add entries with [`url`](Sitemap::url), which takes a path string or a [`SitemapUrl`] carrying the optional fields, and [`urls`](Sitemap::urls), which adds every entry of an iterator, such as one built from the rows of a database query.

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

- [`last_modified`](SitemapUrl::last_modified) is the time the page last changed. It accepts anything convertible into a `SystemTime`, which covers the timestamp types of the common date and time crates.
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

A module-derived path cannot contain a dot, because module names are converted to kebab-case. To serve the sitemap from a module tree, declare a `sitemap` module and override its segment with [`segment!`](macro@crate::segment); a rename is used as written.

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
