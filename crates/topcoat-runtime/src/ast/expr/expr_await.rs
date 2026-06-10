use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::ExprAwait;

use crate::ast::expr::{Expr, NameResolver};

impl Expr {
    pub(super) fn expr_await(
        expr: &ExprAwait,
        rust: &mut TokenStream,
        js: &mut String,
        names: &mut NameResolver,
    ) -> syn::Result<()> {
        *js += "(await ";
        Self::dispatch(&expr.base, rust, js, names)?;
        expr.dot_token.to_tokens(rust);
        expr.await_token.to_tokens(rust);
        *js += ")";
        Ok(())
    }
}
