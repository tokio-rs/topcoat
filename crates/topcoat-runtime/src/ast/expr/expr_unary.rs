use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{ExprUnary, UnOp};

use crate::ast::expr::{Expr, name_resolver::NameResolver};

impl Expr {
    pub(super) fn expr_unary(
        unary: &ExprUnary,
        rust: &mut TokenStream,
        js: &mut String,
        names: &mut NameResolver,
    ) -> syn::Result<()> {
        let op = match unary.op {
            UnOp::Deref(_) => "deref",
            UnOp::Not(_) => "not",
            UnOp::Neg(_) => "neg",
            other => return Err(syn::Error::new_spanned(other, "unsupported operator")),
        };

        let mut left = TokenStream::new();
        Self::dispatch(&unary.expr, &mut left, js, names)?;

        js.push('.');
        js.push_str(op);
        js.push_str("()");

        let op = &unary.op;
        quote! { #op #left }.to_tokens(rust);
        Ok(())
    }
}
