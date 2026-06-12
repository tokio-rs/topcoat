use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{BinOp, ExprBinary, spanned::Spanned};

use crate::ast::expr::{Expr, name_resolver::NameResolver};

enum OpKind {
    Arithmetic,
    Cmp,
}

impl Expr {
    pub(super) fn expr_binary(
        binary: &ExprBinary,
        rust: &mut TokenStream,
        js: &mut String,
        names: &mut NameResolver,
    ) -> syn::Result<()> {
        let (method, kind) = match binary.op {
            BinOp::Add(_) => ("add", OpKind::Arithmetic),
            BinOp::Sub(_) => ("sub", OpKind::Arithmetic),
            BinOp::Mul(_) => ("mul", OpKind::Arithmetic),
            BinOp::Div(_) => ("div", OpKind::Arithmetic),
            BinOp::Eq(_) => ("eq", OpKind::Cmp),
            BinOp::Ne(_) => ("ne", OpKind::Cmp),
            BinOp::Lt(_) => ("lt", OpKind::Cmp),
            BinOp::Le(_) => ("le", OpKind::Cmp),
            BinOp::Gt(_) => ("gt", OpKind::Cmp),
            BinOp::Ge(_) => ("ge", OpKind::Cmp),
            other => return Err(syn::Error::new_spanned(other, "unsupported operator")),
        };

        let mut left = TokenStream::new();
        Self::dispatch(&binary.left, &mut left, js, names)?;

        js.push('.');
        js.push_str(method);
        js.push('(');

        let mut right = TokenStream::new();
        Self::dispatch(&binary.right, &mut right, js, names)?;
        js.push(')');

        match kind {
            OpKind::Arithmetic => {
                let op = &binary.op;
                quote! { #left #op #right }.to_tokens(rust);
            }
            OpKind::Cmp => {
                let method_ident = syn::Ident::new(method, binary.op.span());
                quote! { (#left).#method_ident(&#right) }.to_tokens(rust);
            }
        }
        Ok(())
    }
}
