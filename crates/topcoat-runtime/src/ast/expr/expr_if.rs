use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Expr as SynExpr, ExprIf};

use crate::ast::expr::{Expr, name_resolver::NameResolver};

impl Expr {
    /// Lowers `if cond { ... } else { ... }`. The JavaScript side is wrapped
    /// in an IIFE so the same shape works in both expression and statement
    /// position.
    pub(super) fn expr_if(
        if_expr: &ExprIf,
        rust: &mut TokenStream,
        js: &mut String,
        names: &mut NameResolver,
    ) -> syn::Result<()> {
        js.push_str("(() => ");
        let rust_if = Self::expr_if_inner(if_expr, js, names)?;
        js.push_str(")()");
        rust_if.to_tokens(rust);
        Ok(())
    }

    fn expr_if_inner(
        if_expr: &ExprIf,
        js: &mut String,
        names: &mut NameResolver,
    ) -> syn::Result<TokenStream> {
        js.push_str("{ if (");
        let mut cond = TokenStream::new();
        Self::dispatch(&if_expr.cond, &mut cond, js, names)?;
        js.push_str(".valueOf()) ");

        let mut then_tokens = TokenStream::new();
        Self::block(&if_expr.then_branch, &mut then_tokens, js, names)?;

        let mut else_tokens = TokenStream::new();
        let else_kw = if let Some((else_token, else_branch)) = &if_expr.else_branch {
            js.push_str(" else ");
            match &**else_branch {
                SynExpr::If(inner) => {
                    let inner_rust = Self::expr_if_inner(inner, js, names)?;
                    inner_rust.to_tokens(&mut else_tokens);
                }
                SynExpr::Block(block) => {
                    Self::block(&block.block, &mut else_tokens, js, names)?;
                }
                other => {
                    return Err(syn::Error::new_spanned(other, "unsupported else branch"));
                }
            }
            Some(else_token)
        } else {
            None
        };
        js.push_str(" }");

        let if_token = &if_expr.if_token;
        let rust = if let Some(else_kw) = else_kw {
            quote! {
                #if_token ::topcoat::runtime::Surrogate::into_real(#cond)
                #then_tokens
                #else_kw #else_tokens
            }
        } else {
            quote! {
                #if_token ::topcoat::runtime::Surrogate::into_real(#cond)
                #then_tokens
            }
        };
        Ok(rust)
    }
}
