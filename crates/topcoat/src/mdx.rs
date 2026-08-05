#![doc = include_str!("../docs/mdx.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]

// Called by generated `mdx_pages!` and `mdx_page!` output, which resolves
// paths through this facade. Not a public API.
#[cfg(feature = "mdx-frontmatter")]
#[doc(hidden)]
pub use topcoat_mdx::__private;
pub use topcoat_mdx::{MdxFrontmatterFormat, MdxIndexEntry, mdx_components};
pub use topcoat_mdx_macro::{compile_mdx, mdx_page, mdx_pages};
