use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{BinOp, ExprBinary};

use crate::ast::expr::{Expr, name_resolver::NameResolver};

impl Expr {
    pub(super) fn expr_binary(
        binary: &ExprBinary,
        rust: &mut TokenStream,
        js: &mut String,
        names: &mut NameResolver,
    ) -> syn::Result<()> {
        let op = match binary.op {
            BinOp::Add(_) => "add",
            BinOp::Sub(_) => "sub",
            BinOp::Mul(_) => "mul",
            BinOp::Div(_) => "div",
            other => return Err(syn::Error::new_spanned(other, "unsupported operator")),
        };

        let mut left = TokenStream::new();
        Self::dispatch(&binary.left, &mut left, js, names)?;

        js.push('.');
        js.push_str(op);
        js.push('(');

        let mut right = TokenStream::new();
        Self::dispatch(&binary.right, &mut right, js, names)?;
        js.push(')');

        let op = &binary.op;
        quote! { #left #op #right }.to_tokens(rust);
        Ok(())
    }
}
