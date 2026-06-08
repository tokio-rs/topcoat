use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::ExprContinue;

use crate::ast::expr::{Expr, NameResolver};

impl Expr {
    pub(super) fn expr_continue(
        expr: &ExprContinue,
        rust: &mut TokenStream,
        js: &mut String,
        _names: &mut NameResolver,
    ) -> syn::Result<()> {
        if let Some(label) = &expr.label {
            return Err(syn::Error::new_spanned(label, "labels are not supported"));
        }
        js.push_str("continue");
        expr.to_tokens(rust);
        Ok(())
    }
}
