The `compile_mdx!` macro reads a `.mdx` or `.md` file at compile time, parses it with `markdown-rs`, walks the syntax tree into `view!` AST nodes, and emits the tokens. There is zero runtime parsing overhead.

The macro expands to the same expression a `view!` block produces, so it takes the place of a `view!` block in a handler body rather than nesting inside one.

```rust,ignore
use topcoat::{mdx::compile_mdx, router::page};

#[page("/blog/post")]
async fn post_page() -> topcoat::Result {
    compile_mdx!(
        mdx_components! {
            Callout => components::callout,
        },
        "content/post.mdx"
    )
}
```

# Syntax

The macro accepts four input patterns, from simplest to most feature-rich.

## One-arg form

The simplest form takes only a file path string literal:

```rust,ignore
compile_mdx!("content/post.mdx")
```

This compiles the file with no component registry and no overrides. Embedded component tags like `<Callout>` will not be recognized. Use this form for plain `.mdx` files that do not need custom components.

## Two-arg form with braced block

Pass an inline component registry followed by the file path:

```rust,ignore
compile_mdx!(
    {
        Callout => components::callout,
        Divider => components::divider,
    },
    "content/post.mdx"
)
```

Each entry maps an identifier (used as a tag name in the `.mdx` file) to a Rust component path.

## Two-arg form with `mdx_components!`

The recommended form uses the `mdx_components!` macro to declare the registry:

```rust,ignore
compile_mdx!(
    mdx_components! {
        Callout => components::callout,
        Divider => components::divider,
    },
    "content/post.mdx"
)
```

The registry is read as tokens at compile time, so it must appear in the invocation itself. See [`mdx_components!`][] for details.

## With overrides

Add `overrides = { ... }` after the component registry to map HTML elements to components. Content authors write normal markdown; the framework renders the elements through your components:

```rust,ignore
compile_mdx!(
    mdx_components! {
        Callout => components::callout,
    },
    overrides = {
        "a" => components::custom_link,
        "h1" => components::heading,
    },
    "content/post.mdx"
)
```

The following HTML elements can be overridden: `a`, `h1` through `h6`, `img`, `pre`, `hr`, `p`, `strong`, `blockquote`, `code` (inline code only -- fenced code blocks are overridden via `pre`), `ul`, `ol`, `li`, `table`, `th`, and `td`. Unknown reference tags produce a compile error.

## With wrapper

Add `wrapper = Path` after the overrides (or after the component registry if there are no overrides) to wrap the compiled content in a layout component:

```rust,ignore
compile_mdx!(
    mdx_components! {
        Callout => components::callout,
    },
    wrapper = components::blog_layout,
    "content/post.mdx"
)
```

The wrapper receives the compiled content as a `child` prop. This requires a `__cx: &Cx` variable in scope from the enclosing `view!` call.

# Features

The following MDX features are available through `compile_mdx!`.

## Reference links

Reference-style links and images are resolved from definition declarations in the document. Content authors write `[link text][ref-id]` and `[ref-id]: url "Title"` anywhere in the file. Unknown references produce a compile error.

## Footnotes

Footnote definitions are collected during parsing and rendered as a numbered section at the end of the document. References like `[^id]` become superscript links. Backlinks return to the reference point.

## Heading IDs

Each heading element receives an `id` attribute generated from its text content. Duplicate headings get `-1`, `-2` suffixes to keep IDs unique.

## Code block meta

Fenced code block meta strings are parsed and attached as `data-*` attributes on the `<pre>` element: `data-lang` for the language identifier, `data-lines` for line highlight ranges, `data-title` for the block title, and `data-emphasis` for search terms.

# File Extensions

Both extensions are parsed with the same MDX grammar; the extension is a naming convention, not a parser switch. Component tags work in `.md` files too, and MDX syntax rules (such as `{/* text */}` comments) apply to both. Both extensions are accepted by this macro.

[`mdx_components!`]: macro.mdx_components.html
