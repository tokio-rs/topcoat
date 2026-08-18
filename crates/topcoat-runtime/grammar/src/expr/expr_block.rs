use proc_macro2::TokenStream;
use syn::ExprBlock;

use crate::expr::{Expr, contains_await::ContainsAwait, name_resolver::NameResolver};

impl Expr {
    pub(super) fn expr_block(
        block: &ExprBlock,
        rust: &mut TokenStream,
        js: &mut String,
        names: &mut NameResolver,
    ) -> syn::Result<()> {
        // A Rust block is already an expression; JavaScript has no block
        // expression, so it is wrapped in an immediately-invoked arrow
        // function. This expression can also hold async expressions.
        let is_async = ContainsAwait::in_block(block);
        let predicate = if is_async {
            "(await (async () => "
        } else {
            "(() => "
        };
        js.push_str(predicate);
        Self::block(&block.block, rust, js, names)?;
        js.push_str(if is_async { ")())" } else { ")()" });
        Ok(())
    }
}
