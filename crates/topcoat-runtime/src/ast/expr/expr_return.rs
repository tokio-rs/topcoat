use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::ExprReturn;

use crate::ast::expr::{Expr, NameResolver};

impl Expr {
    pub(super) fn expr_return(
        expr: &ExprReturn,
        rust: &mut TokenStream,
        js: &mut String,
        names: &mut NameResolver,
    ) -> syn::Result<()> {
        expr.return_token.to_tokens(rust);
        js.push_str("return");
        if let Some(expr) = &expr.expr {
            js.push(' ');
            Self::dispatch(expr, rust, js, names)?;
        }
        Ok(())
    }
}
