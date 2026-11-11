use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::Block;

use super::js::Js;
use crate::expr::{Expr, name_resolver::NameResolver};

impl Expr {
    /// Lowers the contents of a `{ ... }` block. The trailing expression (a
    /// statement without a semicolon) becomes the block's value.
    pub(super) fn block(
        block: &Block,
        rust: &mut TokenStream,
        js: &mut Js,
        names: &mut NameResolver,
    ) -> syn::Result<()> {
        Self::block_with_value(block, rust, js, names, true)
    }

    /// Lowers the contents of a `{ ... }` block whose value is discarded.
    pub(super) fn block_stmt(
        block: &Block,
        rust: &mut TokenStream,
        js: &mut Js,
        names: &mut NameResolver,
    ) -> syn::Result<()> {
        Self::block_with_value(block, rust, js, names, false)
    }

    /// Lowers a block with an explicit choice about whether its JavaScript
    /// value is used. The Rust tokens always preserve the block's value so
    /// type checking matches the source expression.
    pub(super) fn block_with_value(
        block: &Block,
        rust: &mut TokenStream,
        js: &mut Js,
        names: &mut NameResolver,
        value_used: bool,
    ) -> syn::Result<()> {
        js.push_str("{ ");
        names.push_scope();

        let mut stmts = TokenStream::new();
        let last = block.stmts.len().wrapping_sub(1);
        for (i, stmt) in block.stmts.iter().enumerate() {
            Self::stmt(stmt, &mut stmts, js, names, i == last, value_used)?;
        }

        names.pop_scope();
        js.push_str(" }");
        quote! { { #stmts } }.to_tokens(rust);
        Ok(())
    }
}
