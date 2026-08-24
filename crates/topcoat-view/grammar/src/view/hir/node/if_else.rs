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

        // The branches build streams of different types; `Either` unifies
        // them, and only the taken branch is driven as this position's unit.
        // The then stream carries the bindings of the condition's `let`
        // patterns, which die with the branch it is created in.
        let then_branch = then_branch.emit_view_captured(&Bindings::of_condition(expr));
        let else_branch = else_branch.emit_view();

        emitter.hoist(quote! {
            let #ident = if #expr {
                #topcoat_view::internal::Either::Left(#then_branch)
            } else {
                #topcoat_view::internal::Either::Right(#else_branch)
            };
        });
        emitter.unit(Span::call_site(), &ident);
    }
}
