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
                Self::expr_stmt(is_last && semi.is_none(), rust, js, |value, js| {
                    Self::dispatch(expr, value, js, names)
                })?;
            }
            Stmt::Macro(stmt_macro) => {
                let returns = is_last && stmt_macro.semi_token.is_none();
                Self::expr_stmt(returns, rust, js, |value, js| {
                    Self::stmt_macro(stmt_macro, value, js, names)
                })?;
            }
            other @ Stmt::Item(_) => {
                return Err(syn::Error::new_spanned(other, "unsupported statement"));
            }
        }
        Ok(())
    }

    /// Lowers an expression statement with `lower`. A trailing expression with
    /// no semicolon (`returns`) is the block's value, so it becomes the
    /// JavaScript `return`; any other ends with its own semicolon.
    fn expr_stmt(
        returns: bool,
        rust: &mut TokenStream,
        js: &mut Js,
        lower: impl FnOnce(&mut TokenStream, &mut Js) -> syn::Result<()>,
    ) -> syn::Result<()> {
        if returns {
            js.push_str("return ");
        }

        let mut value = TokenStream::new();
        lower(&mut value, js)?;

        if returns {
            js.push(';');
            value.to_tokens(rust);
        } else {
            js.push_str("; ");
            quote! { #value; }.to_tokens(rust);
        }
        Ok(())
    }
}
