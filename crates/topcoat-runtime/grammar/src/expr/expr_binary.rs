use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{BinOp, ExprBinary, spanned::Spanned};

use crate::expr::{Expr, name_resolver::NameResolver};

enum OpKind {
    Arithmetic,
    Cmp,
    /// `&&` and `||`, whose right side is compiled into a closure so that it
    /// is only evaluated when the left side does not already decide the
    /// result. Both languages short-circuit, so evaluating eagerly here would
    /// change what the expression means, not just what it costs.
    Logical,
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
            BinOp::And(_) => ("and", OpKind::Logical),
            BinOp::Or(_) => ("or", OpKind::Logical),
            other => return Err(syn::Error::new_spanned(other, "unsupported operator")),
        };

        let mut left = TokenStream::new();
        Self::dispatch(&binary.left, &mut left, js, names)?;

        js.push('.');
        js.push_str(method);
        js.push('(');
        if matches!(kind, OpKind::Logical) {
            js.push_str("() => ");
        }

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
            OpKind::Logical => {
                let method_ident = syn::Ident::new(method, binary.op.span());
                quote! { (#left).#method_ident(|| #right) }.to_tokens(rust);
            }
        }
        Ok(())
    }
}
