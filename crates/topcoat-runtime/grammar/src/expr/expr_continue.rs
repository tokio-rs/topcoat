use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::ExprContinue;

use super::js::Js;
use crate::expr::{Expr, NameResolver};

impl Expr {
    pub(super) fn expr_continue(
        expr: &ExprContinue,
        rust: &mut TokenStream,
        js: &mut Js,
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
