use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::Stmt;

use crate::ast::expr::{Expr, name_resolver::NameResolver};

impl Expr {
    pub(super) fn stmt(
        stmt: &Stmt,
        rust: &mut TokenStream,
        js: &mut String,
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
                names.bind_local(&ident, name)?;

                quote! { let #pat = #value; }.to_tokens(rust);
            }
            Stmt::Expr(expr, semi) => {
                // A trailing expression (no semicolon) is the block's value, so
                // it becomes the JavaScript `return`.
                let returns = is_last && semi.is_none();
                if returns {
                    js.push_str("return ");
                }

                let mut value = TokenStream::new();
                Self::dispatch(expr, &mut value, js, names)?;

                if returns {
                    js.push(';');
                    value.to_tokens(rust);
                } else {
                    js.push_str("; ");
                    quote! { #value; }.to_tokens(rust);
                }
            }
            other => return Err(syn::Error::new_spanned(other, "unsupported statement")),
        }
        Ok(())
    }
}
