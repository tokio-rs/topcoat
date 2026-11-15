use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::Stmt;

use super::js::Js;
use crate::expr::{
    Expr,
    name_resolver::{LocalBindingKind, NameResolver},
};

impl Expr {
    pub(super) fn stmt(
        stmt: &Stmt,
        rust: &mut TokenStream,
        js: &mut Js,
        names: &mut NameResolver,
        is_last: bool,
    ) -> syn::Result<()> {
        match stmt {
            Stmt::Local(local) => {
                let init = local.init.as_ref().ok_or_else(|| {
                    syn::Error::new_spanned(local, "let binding requires an initializer")
                })?;
                if let Some((_, diverge)) = &init.diverge {
                    return Err(syn::Error::new_spanned(
                        diverge,
                        "let-else is not supported",
                    ));
                }

                js.push_str("let ");
                let mut pat = TokenStream::new();
                let (ident, name) = Self::pat(&local.pat, &mut pat, js, names)?;
                js.push_str(" = ");
                let mut value = TokenStream::new();
                Self::dispatch(&init.expr, &mut value, js, names)?;
                js.push_str("; ");
                names.bind_local(&ident, name, LocalBindingKind::Surrogate)?;

                quote! { let #pat = #value; }.to_tokens(rust);
            }
            Stmt::Expr(expr, semi) => {
                // A trailing expression (no semicolon) is the block's value, so
                // it becomes the JavaScript `return`.
                let returns = is_last && semi.is_none();
                if returns {
                    js.push_str("return ");
                    let mut value = TokenStream::new();
                    Self::dispatch(expr, &mut value, js, names)?;
                    js.push(';');
                    value.to_tokens(rust);
                } else if let syn::Expr::If(if_expr) = expr {
                    // Keep statement-position if out of an IIFE so return,
                    // break, and continue apply to the enclosing function or
                    // loop (see tokio-rs/topcoat#603).
                    let mut value = TokenStream::new();
                    Self::expr_if_as_statement(if_expr, &mut value, js, names)?;
                    js.push_str("; ");
                    quote! { #value; }.to_tokens(rust);
                } else {
                    let mut value = TokenStream::new();
                    Self::dispatch(expr, &mut value, js, names)?;
                    js.push_str("; ");
                    quote! { #value; }.to_tokens(rust);
                }
            }
            Stmt::Macro(stmt_macro) => Self::stmt_macro(stmt_macro, rust, js, names)?,
            other @ Stmt::Item(_) => {
                return Err(syn::Error::new_spanned(other, "unsupported statement"));
            }
        }
        Ok(())
    }
}
