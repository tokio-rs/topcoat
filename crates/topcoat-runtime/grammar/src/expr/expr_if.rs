use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Expr as SynExpr, ExprIf};
use topcoat_core_grammar::common::paths::topcoat_runtime;

use super::js::Js;
use crate::expr::{Expr, contains_await::ContainsAwait, name_resolver::NameResolver};

impl Expr {
    /// Lowers `if cond { ... } else { ... }` in expression position. The
    /// JavaScript side is wrapped in an IIFE so the expression produces a
    /// value. This expression can also hold async expressions.
    ///
    /// Conditionals whose JavaScript value is discarded use `expr_if_inner`
    /// directly, which keeps `return`, `break`, and `continue` in their
    /// enclosing scope.
    pub(super) fn expr_if(
        if_expr: &ExprIf,
        rust: &mut TokenStream,
        js: &mut Js,
        names: &mut NameResolver,
    ) -> syn::Result<()> {
        let is_async = ContainsAwait::in_if(if_expr);
        let predicate = if is_async {
            "(await (async () => "
        } else {
            "(() => "
        };
        js.push_str(predicate);
        let rust_if = Self::expr_if_inner(if_expr, js, names, true)?;
        js.push_str(if is_async { ")())" } else { ")()" });
        rust_if.to_tokens(rust);
        Ok(())
    }

    /// Lowers the conditional without the IIFE wrapper. `value_used` controls
    /// whether the branches produce a JavaScript value or discard their
    /// trailing expression. Rust branches always keep their value so type
    /// checking matches the source expression.
    pub(super) fn expr_if_inner(
        if_expr: &ExprIf,
        js: &mut Js,
        names: &mut NameResolver,
        value_used: bool,
    ) -> syn::Result<TokenStream> {
        js.push_str("{ if (");
        let mut cond = TokenStream::new();
        Self::dispatch(&if_expr.cond, &mut cond, js, names)?;
        js.push_str(".dehydrate()) ");

        let mut then_tokens = TokenStream::new();
        Self::block_with_value(
            &if_expr.then_branch,
            &mut then_tokens,
            js,
            names,
            value_used,
        )?;

        let mut else_tokens = TokenStream::new();
        let else_kw = if let Some((else_token, else_branch)) = &if_expr.else_branch {
            js.push_str(" else ");
            match &**else_branch {
                SynExpr::If(inner) => {
                    let inner_rust = Self::expr_if_inner(inner, js, names, value_used)?;
                    inner_rust.to_tokens(&mut else_tokens);
                }
                SynExpr::Block(block) => {
                    Self::block_with_value(&block.block, &mut else_tokens, js, names, value_used)?;
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
                #if_token #topcoat_runtime::Surrogate::into_real(#cond)
                #then_tokens
                #else_kw #else_tokens
            }
        } else {
            quote! {
                #if_token #topcoat_runtime::Surrogate::into_real(#cond)
                #then_tokens
            }
        };
        Ok(rust)
    }
}
