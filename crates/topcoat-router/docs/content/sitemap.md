XML sitemaps for Topcoat routes.

A [sitemap](https://www.sitemaps.org) lists the URLs of a site so that crawlers can find every page. It can also hold some metadata about each page. This module needs the `sitemap` feature. It provides the [`Sitemap`] response. A route builds a sitemap one entry at a time and returns it, and the response is sent as a sitemap XML document with `Content-Type: application/xml`.

# Serving a sitemap

Crawlers look for the sitemap at `/sitemap.xml`. Add one entry with [`url`](Sitemap::url), which takes a path string or a [`SitemapUrl`] with optional fields. Add many entries with [`urls`](Sitemap::urls), which takes an iterator, such as one built from the rows of a database query.

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

The sitemap format needs absolute URLs, so register the public base URL of the application on the router. An entry given as a root-relative path is resolved against the base URL when the response is rendered. An entry that is already an absolute `http` or `https` URL is used unchanged. Rendering a relative entry without a registered base URL panics.

```rust,no_run
use topcoat::router::Router;

let router = Router::builder().base_url("https://example.com").build();
```

# Entry fields

Besides its location, a [`SitemapUrl`] can hold the optional metadata of the sitemap format. Each builder method replaces the field it sets.

- [`last_modified`](SitemapUrl::last_modified) is the time the page last changed. It accepts anything that converts into a `SystemTime`, which includes the timestamp types of the common date and time crates.
- [`change_frequency`](SitemapUrl::change_frequency) tells crawlers how often to visit the page again. It ranges from [`Always`](ChangeFrequency::Always), for a page that changes on every visit, to [`Never`](ChangeFrequency::Never), for an archived page.
- [`priority`](SitemapUrl::priority) ranks the page against the other pages of the site, from `0.0` to `1.0`. Crawlers treat an entry without a priority as `0.5`.

```rust
use std::time::SystemTime;

use topcoat::router::content::sitemap::{ChangeFrequency, SitemapUrl};

let url = SitemapUrl::new("/posts/42")
    .last_modified(SystemTime::now())
    .change_frequency(ChangeFrequency::Weekly)
    .priority(0.8);
```

# The path under `module_router!`

A module name cannot produce a path with a dot in it, because module names are converted to kebab-case. To serve the sitemap from the module tree, declare a `sitemap` module and change its segment with `segment!`. A rename is used exactly as written.

```rust
// src/app/sitemap.rs: serves /sitemap.xml
use topcoat::{Result, router::{content::sitemap::Sitemap, route}};

topcoat::router::segment!(rename = "sitemap.xml");

#[route(GET)]
async fn sitemap() -> Result<Sitemap> {
    Ok(Sitemap::new().url("/"))
}
```

The route with an absolute path from the first example also works with `module_router!`. Register it on the builder by name, or let `discover` collect it.
