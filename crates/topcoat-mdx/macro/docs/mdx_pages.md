The `mdx_pages!` macro scans a directory for `.mdx` and `.md` files, compiles each one at build time into `view!` AST nodes, and registers a page route per file. It also emits a const index array and accessor function for content indexing purposes.

```rust,ignore
use topcoat::{mdx::mdx_pages, router::Router, router::RouterBuilderDiscoverExt};

mdx_pages!("content/blog", prefix = "/blog");

let router = Router::builder().discover().build();
```

`mdx_pages!` must be placed at module level, since it generates consts, functions, and inventory registrations that cannot appear inside a function body.

# Combining with `module_router!`

`mdx_pages!` registers each page as a `PageFn` in the link-time inventory. When using `module_router!`, call `.discover()` on the returned builder so that these inventory items are picked up:

```rust,ignore
use topcoat::router::{module_router, RouterBuilderDiscoverExt};

pub fn router() -> Router {
    let builder: RouterBuilder = module_router!().into();
    builder.discover().build()
}
```

Without `.discover()`, the `#[page]`, `#[layout]`, and `#[route]` items in your module tree work fine, but any pages registered by `mdx_pages!` will not appear on the router.

# Syntax

```text
mdx_pages!(directory_path [, prefix = "/path"] [, components = {...}] [, overrides = {...}] [, wrapper = Path] [, frontmatter = Type])
```

The `directory_path` argument is a required string literal. All remaining arguments are optional and may appear in any order.

## Directory path

A string literal pointing to the directory, relative to `CARGO_MANIFEST_DIR`. All `.mdx` and `.md` files within this directory are scanned recursively. Files matching `.gitignore` patterns are excluded:

```rust,ignore
mdx_pages!("content/blog");
```

## Route prefix

Pass `prefix = "/path"` to prepend a route path segment to each derived route:

```rust,ignore
mdx_pages!("content/blog", prefix = "/blog");
```

This would register `content/blog/hello-world.mdx` at `/blog/hello-world`.

## Route derivation

Route paths are derived from file structure relative to the scan directory. File stems are converted to kebab-case. For example:

| File path | Derived route (no prefix) | Derived route (`prefix = "/blog"`) |
|---|---|---|
| `hello-world.mdx` | `/hello-world` | `/blog/hello-world` |
| `nested/post.mdx` | `/nested/post` | `/blog/nested/post` |
| `my-post/index.mdx` | `/my-post` | `/blog/my-post` |

## Index files

A file named `index.mdx` or `index.md` stands for the directory holding it, so `posts/my-post/index.mdx` serves `/blog/my-post` rather than `/blog/my-post/my-post`. This lets a post keep its images, partials, or translations in a directory of its own without the route repeating itself.

Its slug follows the same rule: the entry above is indexed as `my-post`. Every index file would otherwise answer to `index` and collide with the others.

Nothing else changes. A flat `hello-world.mdx` and a sibling `my-post/appendix.mdx` keep the routes they always had, so both layouts can live in one directory.

Two files that derive the same route are a compile error naming both. This happens when an index file sits beside a same-named sibling, and also when kebab-casing maps two names onto one route, as `my_post.mdx` and `my-post.mdx` do.

## Shared components

Pass `components = {...}` to supply a component registry that applies to all pages in the directory:

```rust,ignore
mdx_pages!(
    "content/blog",
    components = {
        Callout => components::callout,
    }
);
```

The registry applies to every page in the scan, so a component used by several files is declared once.

## Shared overrides

Pass `overrides = { ... }` to replace HTML elements with components across all pages:

```rust,ignore
mdx_pages!(
    "content/blog",
    overrides = { "a" => components::custom_link }
);
```

## Shared wrapper

Pass `wrapper = Path` to wrap all pages in the same layout component:

```rust,ignore
mdx_pages!("content/blog", wrapper = components::blog_layout);
```

The wrapper receives the compiled page as its `child` prop, so it must declare one. It must not require any other prop, apart from `meta` when `frontmatter = Type` is also given.

## Typed frontmatter

Pass `frontmatter = Type` to have the macro deserialize each page's frontmatter itself. It picks the deserializer from the syntax each page used, parses once on first read, and hands the result to both the index and the wrapper:

```rust,ignore
use topcoat::{Result, mdx::mdx_pages, view::{View, component, view}};

#[derive(serde::Deserialize)]
struct PostMeta {
    subtitle: Option<String>,
    #[serde(rename = "publishDate")]
    publish_date: String,
}

#[component]
async fn blog_layout(#[default] child: View, meta: Option<&'static PostMeta>) -> Result {
    view! {
        <article>
            if let Some(meta) = meta {
                <p>(&meta.publish_date)</p>
            }
            (child)
        </article>
    }
}

mdx_pages!(
    "content/blog",
    frontmatter = PostMeta,
    wrapper = blog_layout,
);
```

Entries then answer `meta()`, and sorting or filtering on custom fields needs no deserializing of your own:

```rust,ignore
let mut posts: Vec<_> = mdx_index_content_blog().iter().collect();
posts.sort_by_key(|post| post.meta().map(|meta| &meta.publish_date));
```

`meta` is an `Option` on both the index entry and the wrapper prop. A directory may hold pages that carry no frontmatter, and those pass `None` rather than being rejected.

This needs the `mdx-frontmatter` feature of `topcoat`, or the `frontmatter` feature of `topcoat-mdx` directly. Rendering MDX resolves frontmatter while the macro expands, on the build machine; reading it into a type of your own happens at runtime, and that is what the feature adds.

### What is not checked

The macro sees the name of the type, never its fields, and serde does not run while the macro expands. A page whose frontmatter does not match the type is therefore not a compile error: it panics on first read, naming the file. Frontmatter that is not valid YAML or TOML at all is still a compile error, as before.

# Content Indexer

The macro emits two artifacts for content indexing: a const array and an accessor function.

## Index const

A `&'static [MdxIndexEntry]` const named `MDX_INDEX_{DIR}` is emitted, where `{DIR}` is the directory path converted to uppercase with separators replaced by underscores. For example, scanning `"content/blog"` produces `MDX_INDEX_CONTENT_BLOG`.

Each `MdxIndexEntry` contains the following fields populated from frontmatter and file metadata:

- `slug`: the kebab-cased route slug derived from the file stem
- `path`: the full route path, including the prefix and any subdirectories
- `title`: the `title` field from frontmatter, if present
- `date`: the `date` field from frontmatter, if present
- `excerpt`: the `excerpt` field from frontmatter, if present
- `tags`: the `tags` field from frontmatter as a slice of strings, empty if absent
- `frontmatter_raw`: the whole frontmatter block, delimiters stripped, empty when the page has none
- `frontmatter_format`: whether that block is YAML, TOML, or absent
- `word_count`: words in the page body, counted at compile time without the frontmatter

Entries also answer `meta()`, which holds the frontmatter deserialized into the type passed as `frontmatter = Type`, and `None` when that argument was not given.

## Custom frontmatter fields

Pages often carry more than the four named fields. Deserialize `frontmatter_raw` into a type of your own to read the rest, choosing the deserializer from `frontmatter_format`:

```rust,ignore
use topcoat::mdx::{MdxFrontmatterFormat, MdxIndexEntry, mdx_pages};

mdx_pages!("content/blog", prefix = "/blog");

#[derive(serde::Deserialize)]
struct PostMeta {
    subtitle: Option<String>,
    #[serde(rename = "publishDate")]
    publish_date: String,
}

fn meta(entry: &MdxIndexEntry) -> Option<PostMeta> {
    match entry.frontmatter_format {
        MdxFrontmatterFormat::Yaml => serde_saphyr::from_str(entry.frontmatter_raw).ok(),
        MdxFrontmatterFormat::Toml => toml::from_str(entry.frontmatter_raw).ok(),
        MdxFrontmatterFormat::None => None,
    }
}
```

The parser strips the `---` and `+++` delimiters, so the syntax cannot be told from the string itself. Always read `frontmatter_format` rather than inspecting the text.

Deserializing happens at runtime, once per call. Cache the result in a `LazyLock` if a listing page reads it for every request.

## Reading time

`word_count` counts whitespace-separated words in the body, so a reading estimate is a division by whatever rate suits your audience:

```rust,ignore
let minutes = entry.word_count.div_ceil(200);
```

Code blocks and component markup count toward the total, matching what reading-time tooling reports for a markdown file.

## Index accessor function

A function named `mdx_index_{dir}` is emitted, where `{dir}` is the lowercase directory path with separators replaced by underscores. For `"content/blog"`, the accessor is `mdx_index_content_blog()`:

```rust,ignore
use topcoat::{mdx::mdx_pages, Result, router::page, view::view};

mdx_pages!("content/blog", prefix = "/blog");

#[page]
async fn blog_index() -> Result {
    let entries = mdx_index_content_blog();
    view! {
        <ul>
            for entry in entries {
                <li>
                    <a href=(entry.path)>(entry.title.unwrap_or(entry.slug))</a>
                </li>
            }
        </ul>
    }
}
```

# File Extensions

Both extensions are scanned and parsed with the same MDX grammar; the extension is a naming convention, not a parser switch. Component tags work in `.md` files too.

[`compile_mdx!`]: macro.compile_mdx.html
[`MdxIndexEntry`]: struct.MdxIndexEntry.html
