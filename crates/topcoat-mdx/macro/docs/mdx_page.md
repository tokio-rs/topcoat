The `mdx_page!` macro compiles a single `.mdx` or `.md` file and registers it as a page route. The macro reads the file at compile time, parses it with `markdown-rs`, walks the syntax tree into `view!` AST nodes, and submits the route to the inventory so that `Router::builder().discover()` picks it up.

```rust,ignore
use topcoat::{mdx::mdx_page, router::Router, router::RouterBuilderDiscoverExt};

mdx_page!("/blog/hello", "content/hello.mdx");

let router = Router::builder().discover().build();
```

# Syntax

```text
mdx_page!(route_path, file_path [, components = {...}] [, overrides = {...}] [, wrapper = Path])
```

The `route_path` and `file_path` arguments are required string literals. All remaining arguments are optional and may appear in any order.

## Route path

A string literal specifying the URL path for this page:

```rust,ignore
mdx_page!("/blog/hello", "content/hello.mdx");
```

## File path

A string literal pointing to the `.mdx` or `.md` file, relative to `CARGO_MANIFEST_DIR`:

```rust,ignore
mdx_page!("/about", "pages/about.mdx");
```

## Components

Pass `components = {...}` to supply an inline component registry:

```rust,ignore
mdx_page!(
    "/blog/hello",
    "content/hello.mdx",
    components = {
        Callout => components::callout,
        Divider => components::divider,
    }
);
```

Alternatively, pass an `mdx_components!{...}` invocation in place of the braced block. See [`compile_mdx!`][] for the different registry forms.

## Overrides

Pass `overrides = { ... }` to replace HTML elements with components:

```rust,ignore
mdx_page!(
    "/blog/hello",
    "content/hello.mdx",
    overrides = {
        "a" => components::custom_link,
        "h1" => components::heading,
    }
);
```

The following HTML elements can be overridden: `a`, `h1` through `h6`, `img`, `pre`, `hr`, `p`, `strong`, `blockquote`, `code` (inline code only -- fenced code blocks are overridden via `pre`), `ul`, `ol`, `li`, `table`, `th`, and `td`. When a link or image element is overridden, URL safety checks run before the override component is invoked.

## Wrapper

Pass `wrapper = Path` to wrap the compiled content in a layout component:

```rust,ignore
mdx_page!(
    "/blog/hello",
    "content/hello.mdx",
    wrapper = components::blog_layout
);
```

The wrapper receives the compiled content as a `child` prop, and must not require any other prop apart from `meta` when `frontmatter = Type` is also given.

## Typed frontmatter

Pass `frontmatter = Type` to deserialize the page's frontmatter into a type of your own, once on first read, and hand it to the wrapper as a `meta` prop:

```rust,ignore
#[derive(serde::Deserialize)]
struct PostMeta {
    subtitle: Option<String>,
}

mdx_page!(
    "/blog/hello",
    "content/hello.mdx",
    wrapper = components::blog_layout,
    frontmatter = PostMeta
);
```

The prop is `Option<&'static PostMeta>`, holding `None` when the page carries no frontmatter.

This needs the `mdx-frontmatter` feature of `topcoat`, or the `frontmatter` feature of `topcoat-mdx` directly, since deserializing into your type happens at runtime rather than while the macro expands. A page whose frontmatter does not match the type panics on first read, naming the file: the macro sees only the type's name and cannot check the two against each other. See the [`mdx_pages!`] reference for the same argument applied to a whole directory.

# Features

The following features are available when compiling the page.

## Heading IDs

Each heading element receives an `id` attribute generated from its text content. Duplicate headings get `-1`, `-2` suffixes. When combined with an `h1` through `h6` override, the component receives the `id` attribute as input.

## Reference links

Reference-style links and images are resolved from definition declarations in the document. Unknown references produce a compile error.

## Footnotes

Footnote definitions are collected and rendered as a numbered section at the end of the document. References become superscript links with backlinks.

## Code block meta

Fenced code block meta strings are parsed and attached as `data-*` attributes on the `<pre>` element: `data-lang`, `data-lines`, `data-title`, and `data-emphasis`.

# File Extensions

Both extensions are parsed with the same MDX grammar; the extension is a naming convention, not a parser switch. Component tags work in `.md` files too, and MDX syntax rules (such as `{/* text */}` comments) apply to both.

[`compile_mdx!`]: macro.compile_mdx.html
[`mdx_pages!`]: macro.mdx_pages.html
