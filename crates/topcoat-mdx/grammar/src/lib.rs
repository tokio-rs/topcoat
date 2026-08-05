//! Grammar crate for `topcoat-mdx`.
//!
//! Contains the `markdown-rs` parser configuration and the mdast-to-view AST
//! walker that transforms markdown nodes into Topcoat `view!` AST types.

#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod parse;
pub mod walker;
