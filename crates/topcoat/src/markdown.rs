#![doc = include_str!("../docs/markdown.md")]

#[expect(
    unused_imports,
    reason = "the markdown crates do not export any items yet"
)]
pub use topcoat_markdown::*;
#[expect(
    unused_imports,
    reason = "the markdown crates do not export any items yet"
)]
pub use topcoat_markdown_macro::*;
