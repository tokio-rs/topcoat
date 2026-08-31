use proc_macro2::Span;
use quote::quote;
use syn::Expr;
use topcoat_core_grammar::paths::topcoat_view;

use crate::view::hir::{
    Bindings, Scope,
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

        if then_branch.is_async() || else_branch.is_async() {
            // The branches build views of different types; `EitherView`
            // unifies them, and only the taken branch is driven as this
            // position's unit. The then branch takes the bindings of the
            // condition's `let` patterns with it; the else branch binds
            // nothing.
            let then_branch = then_branch.emit_captured(&Bindings::of_condition(expr));
            let else_branch = else_branch.emit_inert();

            emitter.hoist(quote! {
                let #ident = if #expr {
                    #topcoat_view::internal::EitherView::left(#then_branch)
                } else {
                    #topcoat_view::internal::EitherView::right(#else_branch)
                };
            });
            emitter.unit(Span::call_site(), &ident);
        } else {
            // The taken branch builds its block right inside the branch,
            // where the condition's bindings are alive. An empty else
            // branch still yields the empty view, so both branches produce
            // a handle to splice.
            let then_branch = then_branch.emit_block();
            let else_branch = else_branch.emit_block();

            emitter.hoist(quote! {
                let #ident = if #expr { #then_branch } else { #else_branch };
            });
            emitter.burst(quote! { __b.view(#ident); });
        }
    }
}
