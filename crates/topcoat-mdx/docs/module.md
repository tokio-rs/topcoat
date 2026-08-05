Topcoat MDX compiles `.mdx` and `.md` files at build time into `view!` AST nodes. Content authors write markdown with embedded Topcoat components, and the `compile_mdx!` macro reads the file, parses it with `markdown-rs`, walks the syntax tree into `view!` nodes, and emits tokens. There is zero runtime parsing overhead.

```rust,ignore
use topcoat::{mdx::{compile_mdx, mdx_components}, router::page, view::view};

#[page("/blog/hello")]
async fn hello_page() -> topcoat::Result {
    view! {
        compile_mdx!(
            mdx_components! {
                Callout => components::callout,
            },
            "content/hello.mdx"
        )
    }
}
```

# Setup

Enable the `mdx` feature on the `topcoat` facade crate:

```toml
topcoat = { version = "0.5.0", features = ["mdx"] }
```

The `compile_mdx!` macro resolves file paths relative to `CARGO_MANIFEST_DIR`. The path argument must be a string literal so the macro can read the file at compile time.

# Syntax Support

Topcoat MDX uses `markdown-rs` as its parser, which supports `CommonMark` and the following GFM extensions:

- Tables
- Strikethrough
- Task lists
- Autolinks

HTML passthrough is disabled. Raw HTML blocks and inline HTML tags are not rendered. This is a security measure: raw `<script>` and `<iframe>` elements cannot slip through the MDX pipeline. Only `<` tokens dispatched to MDX JSX productions are processed.

Both extensions are accepted by `compile_mdx!`, `mdx_page!`, and `mdx_pages!`, and both are parsed with the same MDX grammar. The extension is a naming convention for the reader, not a parser switch: component tags work in a `.md` file too, and MDX syntax rules apply to both. Use `{/* text */}` for comments; an HTML comment is a parse error in either extension.

# Component Embedding

Use `mdx_components!` to declare a registry of component mappings. Each entry pairs an identifier with a Rust component path. When the parser encounters `<Callout>` in an `.mdx` file, it renders the mapped component.

```text
mdx_components! {
    Callout => components::callout,
    Divider => components::divider,
}
```

Pass the registry as the first argument to `compile_mdx!`. The macro reads it as tokens at compile time, so each file declares the components it uses. See the [`mdx_components!`] reference for syntax details, props, children, and self-closing tag support.

# Frontmatter

MDX files can carry YAML or TOML frontmatter at the top of the document, delimited by `---` for YAML or `+++` for TOML. Frontmatter never renders as content.

[`mdx_pages!`] reads the `title`, `date`, `excerpt`, and `tags` fields of every file it scans into a compile-time index. Read that index through the generated accessor to build listings, tag pages, and sitemaps. See the [`mdx_pages!`] reference for the index shape.

Pages are free to declare fields beyond those four. Each index entry carries the whole frontmatter block in `frontmatter_raw`, tagged with the syntax it was written in, so you can deserialize it into a type of your own. See [`MdxIndexEntry`] for the pattern.

Passing `frontmatter = Type` to [`mdx_pages!`] hands that work to the macro instead: it picks the deserializer per page, parses once, and gives both the index entry and the page's wrapper component the result. That part is behind the `frontmatter` feature, since it is the only frontmatter handling that happens at runtime rather than while the macro expands.

# Route Registration

The [`mdx_page!`] macro compiles a single file and registers it as a route handler. The [`mdx_pages!`] macro walks a directory tree, compiles every `.mdx` and `.md` file, and registers a handler per file with kebab-case slugs derived from the filename. A file named `index` is named after the directory holding it instead, and serves that directory's route.

Both macros accept optional `components`, `overrides`, and `wrapper` arguments. See the macro reference docs for details.

[`MdxIndexEntry`]: struct.MdxIndexEntry.html
[`mdx_components!`]: macro.mdx_components.html
[`mdx_page!`]: macro.mdx_page.html
[`mdx_pages!`]: macro.mdx_pages.html
[`compile_mdx!`]: macro.compile_mdx.html
