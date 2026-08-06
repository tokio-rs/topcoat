use proc_macro2::Span;
use quote::quote;
use syn::Expr;

use crate::view::hir::{
    Scope,
    emit::{Emit, Emitter},
};

/// An `if`/`else` whose branches are lowered into nested scopes.
pub(crate) struct IfElse {
    pub expr: Expr,
    pub then_branch: Scope,
    pub else_branch: Scope,
}

impl Emit for IfElse {
    fn emit(&self, emitter: &mut Emitter) {
        let ident = emitter.fresh_ident();
        let Self {
            expr,
            then_branch,
            else_branch,
        } = self;

        let renders_components = then_branch.is_async() || else_branch.is_async();
        if emitter.inline_await() || !renders_components {
            let then_branch = then_branch.emit_expr();
            // An empty else branch still yields the empty view, so both
            // branches produce a view to splice.
            let else_branch = else_branch.emit_expr();

            emitter.hoist(quote! {
                let #ident = if #expr { #then_branch } else { #else_branch };
            });
        } else {
            // In a joined position the branches yield futures instead of
            // views, so the taken branch joins with the scope's other
            // components. `Either` unifies the two future types.
            let then_branch = then_branch.emit_future();
            let else_branch = else_branch.emit_future();

            emitter.hoist_future(
                Span::call_site(),
                &ident,
                &quote! {
                    if #expr {
                        __Either::Left(#then_branch)
                    } else {
                        __Either::Right(#else_branch)
                    }
                },
            );
        }
        emitter.emit(quote! { __view(__cx, __parts, #ident); });
    }
}
