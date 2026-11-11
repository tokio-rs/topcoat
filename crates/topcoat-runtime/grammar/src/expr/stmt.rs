use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::Stmt;

use super::js::Js;
use crate::expr::{
    Expr,
    name_resolver::{LocalBindingKind, NameResolver},
};

impl Expr {
    /// Lowers one statement. `is_last` marks the block's trailing statement
    /// and `value_used` reports whether the surrounding block's JavaScript
    /// value is used. The Rust tokens always follow the source expression's
    /// value.
    pub(super) fn stmt(
        stmt: &Stmt,
        rust: &mut TokenStream,
        js: &mut Js,
        names: &mut NameResolver,
        is_last: bool,
        value_used: bool,
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
                // A trailing expression (no semicolon) is the Rust block's
                // value. It becomes the JavaScript `return` only when the
                // surrounding block's JavaScript value is used.
                let rust_value = is_last && semi.is_none();
                let js_value = value_used && rust_value;
                let mut tokens = TokenStream::new();
                if js_value {
                    js.push_str("return ");
                    Self::dispatch(expr, &mut tokens, js, names)?;
                    js.push(';');
                    tokens.to_tokens(rust);
                } else {
                    Self::stmt_expr(expr, &mut tokens, js, names)?;
                    js.push_str("; ");
                    if rust_value {
                        tokens.to_tokens(rust);
                    } else {
                        quote! { #tokens; }.to_tokens(rust);
                    }
                }
            }
            Stmt::Macro(stmt_macro) => Self::stmt_macro(stmt_macro, rust, js, names)?,
            other @ Stmt::Item(_) => {
                return Err(syn::Error::new_spanned(other, "unsupported statement"));
            }
        }
        Ok(())
    }

    /// Lowers an expression whose JavaScript value is discarded.
    ///
    /// `if` and block expressions are emitted without the IIFE used in
    /// expression position, so `return`, `break`, and `continue` keep their
    /// enclosing function or loop. Their trailing value is discarded in
    /// JavaScript but kept as the Rust block's value, matching the type
    /// checking of the source expression.
    fn stmt_expr(
        expr: &syn::Expr,
        rust: &mut TokenStream,
        js: &mut Js,
        names: &mut NameResolver,
    ) -> syn::Result<()> {
        match expr {
            syn::Expr::If(if_expr) => {
                Self::expr_if_inner(if_expr, js, names, false)?.to_tokens(rust);
            }
            syn::Expr::Block(block) => Self::block_stmt(&block.block, rust, js, names)?,
            _ => Self::dispatch(expr, rust, js, names)?,
        }
        Ok(())
    }
}
