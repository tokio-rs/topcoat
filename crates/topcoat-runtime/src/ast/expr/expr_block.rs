use proc_macro2::TokenStream;
use syn::ExprBlock;

use crate::ast::expr::{Expr, name_resolver::NameResolver};

impl Expr {
    pub(super) fn expr_block(
        block: &ExprBlock,
        rust: &mut TokenStream,
        js: &mut String,
        names: &mut NameResolver,
    ) -> syn::Result<()> {
        // A Rust block is already an expression; JavaScript has no block
        // expression, so it is wrapped in an immediately-invoked arrow
        // function.
        js.push_str("(() => ");
        Self::block(&block.block, rust, js, names)?;
        js.push_str(")()");
        Ok(())
    }
}
