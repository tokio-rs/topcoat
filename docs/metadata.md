Page metadata gives browsers, search engines, and social networks information about a page, such as its title, description, and preview image. [`Metadata`] holds these values. Layouts provide defaults, and pages replace the fields they need to change.

The `metadata` feature is enabled by default. Read the combined values with [`metadata(cx)`](fn@metadata), or render them as HTML tags with [`tags`].

# Setting metadata

Add `Metadata` to a layout or page with `with(...)`. The router makes these values available before rendering, so an outer layout can read the metadata of the page it wraps:

```rust
use topcoat::{
    Result,
    metadata::Metadata,
    router::{Router, Slot, layout, page},
    view::{View, view},
};

#[layout("/", with(
    Metadata::new()
        .title("Acme")
        .description("Everything you need for tea.")
))]
async fn shell(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html>
            <head>topcoat::metadata::tags()</head>
            <body>(slot)</body>
        </html>
    })
}

#[page("/products", with(Metadata::new().title("Products")))]
async fn products() -> Result<impl View> {
    Ok(view! { <h1>"Products"</h1> })
}

let router = Router::builder().layout(shell).page(products).build();
```

The products page has the title "Products" and the description "Everything you need for tea." Setting the title leaves the inherited description in place.

Metadata combines one field at a time. Router builder values are the defaults, outer layouts can replace them, inner layouts can replace those, and the page has the final say. Values added during a request with `cx.with(...)` take priority for code given that child context. See [route context](crate::context#route-context) for how these values reach the request.

An unset field inherits its value. A removal method such as [`no_description`](Metadata::no_description) blocks inheritance for that field, so a page can omit a description supplied by a layout. Other fields keep their values. The same rule applies to individual fields inside the robots, Open Graph, and X settings. The Open Graph image list is treated as one field and replaced as a whole.

# Reading metadata

Call [`metadata(cx).await?`](fn@metadata) to borrow the combined [`PageMetadata`]. You can read it in a page, layout, component, or helper. For example, a component can use the page title as a heading:

```rust
use topcoat::{
    Result,
    context::Cx,
    metadata::metadata,
    view::{View, component, view},
};

#[component]
async fn page_heading(cx: &Cx) -> Result<impl View> {
    let page = metadata(cx).await?;

    Ok(view! {
        if let Some(title) = page.title() {
            <h1>(title)</h1>
        }
    })
}
```

The accessor combines every `Metadata` value visible in the context. Reading `context::<Metadata>(cx)` would return only the closest entry, without inherited fields.

# Rendering tags

Use [`topcoat::metadata::tags()`](tags) to render the page's combined metadata. Place it first in the layout's `<head>`:

```rust
use topcoat::{
    Result,
    router::{Slot, layout},
    view::{View, view},
};

#[layout("/")]
async fn shell(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html>
            <head>
                topcoat::metadata::tags()
            </head>
            <body>(slot)</body>
        </html>
    })
}
```

The component calls `metadata(cx)` and renders the matching HTML tags. It starts with `<meta charset="utf-8">`, which needs to appear within the document's first 1024 bytes. Fields with no value produce no tag. Some fields have defaults, such as the viewport and Open Graph type, so `tags()` still renders useful tags when no `Metadata` was added.

If you need to choose individual tags or supply custom markup, the components in [`tag`] render each kind separately. Read `metadata(cx).await?` once and pass the result to them, for example `tag::title(metadata: page)`.

# Loading metadata for a request

Values in `with(...)` are created when the router is built. Use [`Metadata::from_fn`] when metadata needs async work or request data, such as a database lookup. It takes an async function that receives `&Cx` and returns `Result<Metadata>`:

```rust
# fn main() {}
use topcoat::{
    Result,
    context::{Cx, memoize},
    metadata::Metadata,
    router::page,
    view::{View, view},
};

# struct Product { name: String, description: String }
# mod db {
#     pub async fn find_product(_: &topcoat::context::Cx) -> topcoat::Result<super::Product> {
#         Ok(super::Product { name: "Teapot".into(), description: "A six-cup teapot.".into() })
#     }
# }
#[memoize(as_ref)]
async fn find_product(cx: &Cx) -> Result<Product> {
    db::find_product(cx).await
}

#[page("/product", with(Metadata::from_fn(async |cx| {
    let product = find_product(cx).await?;
    Ok(Metadata::new()
        .title(product.name.clone())
        .description(product.description.clone()))
})))]
async fn product_page(cx: &Cx) -> Result<impl View> {
    let product = find_product(cx).await?;
    Ok(view! { <h1>(&product.name)</h1> })
}
```

The page and metadata loader share the lookup through [`#[memoize]`](crate::context::memoize). The first caller loads the product, and the other borrows the cached result. See [memoization](crate::context::memoize) for the caching rules.

Loaded fields follow the same inheritance rules as directly set fields. Within one metadata entry, directly set fields take priority. For example, `Metadata::from_fn(load).title("Special offer")` overrides the title returned by `load` while keeping its other fields.

`metadata(cx)` waits for these functions and returns the first error. `tags()` reads through that accessor, so it waits for the metadata before rendering its tags and passes on loading errors.

You do not need `from_fn` for every value that varies by request. Text fields also accept translations from `t!`, which use the locale in the rendering context. With `l10n-fluent` enabled and messages declared through [`messages!`](crate::l10n::fluent::messages), pass the translation directly:

```rust
# topcoat::l10n::fluent::messages!("l10n/{locale}.ftl");
use topcoat::metadata::Metadata;

let metadata = Metadata::new().title(t!(title));
```

See [localization](crate::l10n) for declaring messages and choosing the request's locale. Likewise, [preview images](#images) accept `asset!` directly because asset URLs resolve when rendered.

# Metadata fields

The following sections describe the fields you can set on `Metadata`, their defaults, and the tags they produce.

## Title and description

[`title`](Metadata::title) sets the text of `<title>`. [`description`](Metadata::description) sets `<meta name="description">`, a short summary that search engines may show below the page title:

```rust
use topcoat::metadata::Metadata;

let metadata = Metadata::new()
    .title("Teapots | Acme")
    .description("Browse teapots for everyday use.");
```

Both fields accept strings or translations. Neither has a default. Use [`no_description`](Metadata::no_description) to omit an inherited description.

## Canonical URL

[`canonical`](Metadata::canonical) sets `<link rel="canonical">`. This identifies the preferred URL when the same content is available at several addresses.

Use [`Url::Current`] to use the current request's URL without its query string. This is often a useful layout default:

```rust
use topcoat::{
    metadata::{Metadata, Url},
    router::Router,
};

let router = Router::builder()
    .base_url("https://example.com")
    .with(Metadata::new().canonical(Url::Current))
    .build();
```

You can also pass an absolute URL, a path relative to the site root such as `/products`, or [`Url::route(&handler)`](Url::route) for a page without path parameters. For a route with parameters, build its URL with [`href!`](macro@crate::router::href) in a [loading function](#loading-metadata-for-a-request).

Canonical tags request absolute URLs. Set the router's [base URL](crate::router::RouterBuilder::base_url) so Topcoat can turn `/products` into `https://example.com/products`. URLs that are already absolute stay unchanged. Resolving a site-relative URL without the required base URL panics.

There is no default canonical URL. Use [`no_canonical`](Metadata::no_canonical) to omit an inherited one.

## Viewport

[`viewport`](Metadata::viewport) controls `<meta name="viewport">`, which tells mobile browsers how to size and scale the page. The default is `width=device-width, initial-scale=1`.

Pass a string to change it:

```rust
use topcoat::metadata::Metadata;

let metadata = Metadata::new()
    .viewport("width=device-width, initial-scale=1, viewport-fit=cover");
```

Use [`no_viewport`](Metadata::no_viewport) to omit the tag, including its default value.

## Search engine instructions

[`robots`](Metadata::robots) sets whether search engines should include the page in search results and follow its links:

```rust
use topcoat::metadata::Metadata;

let metadata = Metadata::new().robots(|robots| {
    robots.index(false).follow(false)
});
```

`index(false)` adds `noindex` to the robots tag. `follow(false)` adds `nofollow`. Both settings default to `true`, and the tag is omitted when both are allowed.

The settings inherit separately. For example, a layout can set `index(false)` for a section, and a page can set `follow(false)` while keeping the layout's indexing choice. A page can also use `index(true)` to replace an inherited `false`.

## Theme color

[`theme_color`](Metadata::theme_color) sets `<meta name="theme-color">`. Some browsers use this color for their interface around the page:

```rust
use topcoat::metadata::Metadata;

let metadata = Metadata::new().theme_color("#0f766e");
```

There is no default theme color. Use [`no_theme_color`](Metadata::no_theme_color) to omit an inherited value.

## Open Graph

[`open_graph`](Metadata::open_graph) sets the `og:*` tags used in social link previews. Its fields inherit separately, so a layout can set the website name while a page supplies its own image:

```rust
use topcoat::metadata::{Image, Metadata, OpenGraphType};

let metadata = Metadata::new().open_graph(|og| {
    og.site_name("Acme")
        .kind(OpenGraphType::Article)
        .image(
            Image::new("https://example.com/images/brewing-tea.png")
                .size(1200, 630)
                .alt("A teapot beside two cups of tea"),
        )
});
```

The builder supports these fields:

| Method | Purpose | Value when unset |
| --- | --- | --- |
| [`title`](OpenGraphBuilder::title) | Title shown in the preview | Page title |
| [`description`](OpenGraphBuilder::description) | Summary shown in the preview | Page description |
| [`url`](OpenGraphBuilder::url) | URL that identifies the page | Canonical URL |
| [`site_name`](OpenGraphBuilder::site_name) | Name of the website | None |
| [`kind`](OpenGraphBuilder::kind) | Content type, such as a website or article | [`OpenGraphType::Website`] |
| [`image`](OpenGraphBuilder::image) | Image to show in the preview | No images |

These defaults apply after inheritance. For example, an Open Graph title set by a layout takes priority over the page's regular title. Leave the layout's Open Graph title unset if each preview should use its page's title.

### Images

Pass a URL directly to `image`, or use [`Image`] to add pixel dimensions and alternative text. The title, description, website name, and image alternative text accept translations just like the page title.

Bundled assets work directly too:

```rust
use topcoat::{asset::asset, metadata::Metadata};

let metadata = Metadata::new()
    .open_graph(|og| og.image(asset!("./preview.png")));
```

The asset URL is resolved when the tags render, so this does not need `from_fn`. Set up [asset bundling](crate::asset) as usual. Open Graph page and image URLs request the absolute form, so site-relative URLs also need the router's base URL.

Call `image` several times to add several images to one metadata entry. This list replaces all inherited images. Use [`no_images`](OpenGraphBuilder::no_images) to remove the inherited list without adding a replacement.

## X previews

[`twitter`](Metadata::twitter) sets the `twitter:*` tags for link previews on X. X uses the Open Graph title, description, and images for the preview content. The X settings choose the card format and identify the website and author accounts:

```rust
use topcoat::metadata::{Metadata, TwitterCard};

let metadata = Metadata::new().twitter(|x| {
    x.card(TwitterCard::SummaryLargeImage)
        .site("@acme")
        .creator("@ada")
});
```

[`card`](TwitterBuilder::card) selects the preview format. [`TwitterCard::Summary`] uses a small image beside the text, while [`TwitterCard::SummaryLargeImage`] uses a large image above it. If no card is set or inherited, pages with Open Graph images use the large image format, and pages without images use the summary format.

[`site`](TwitterBuilder::site) identifies the website's account, and [`creator`](TwitterBuilder::creator) identifies the page author's account. Include the `@` prefix. Neither account has a default, and each field inherits separately.
