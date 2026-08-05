The `mdx_components!` macro produces a component registry mapping MDX tag names to Rust component paths. It is a `macro_rules!` whose braced block is consumed by `compile_mdx!`, `mdx_page!`, and `mdx_pages!`.

```text
mdx_components! {
    Callout => crate::components::callout,
    Divider => crate::components::divider,
}
```

Each entry maps an identifier to a Rust component path. The identifier becomes the tag name recognized in `.mdx` files. Trailing commas are supported, and component paths can be fully qualified:

```text
mdx_components! {
    Callout => crate::ui::blog::callout::Callout,
    Admonition => super::components::Admonition,
}
```

# Usage

The registry is only meaningful as an argument to one of the MDX macros, which read it as tokens at compile time. Pass the invocation directly as the first argument:

```rust,ignore
use topcoat::{mdx::{compile_mdx, mdx_components}, router::page};

#[page("/blog/post")]
async fn post_page() -> topcoat::Result {
    compile_mdx!(
        mdx_components! {
            Callout => components::callout,
            Divider => components::divider,
        },
        "content/post.mdx"
    )
}
```

Because the tokens are consumed by the enclosing macro, `mdx_components!` is never expanded on its own and cannot be bound to a variable. Each MDX file declares the components it uses, so a page only resolves the tags it names.

# Component Props

When the parser encounters a component tag in the `.mdx` file, attribute syntax becomes component props:

```mdx
<Callout type="info" title="Note">
This is the callout body content.
</Callout>
```

The `type` and `title` attributes are passed as props to the `callout` component. The body content is passed as the `child` prop.

# Self-Closing Tags

Self-closing component tags are supported:

```mdx
<Divider />
```

This renders the `divider` component with no children.

[`compile_mdx!`]: macro.compile_mdx.html
