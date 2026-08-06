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
        let then_branch = then_branch.emit_expr();
        // An empty else branch still yields the empty view, so both branches
        // produce a view to splice.
        let else_branch = else_branch.emit_expr();

        emitter.hoist(quote! {
            let #ident = if #expr { #then_branch } else { #else_branch };
        });
        emitter.emit(quote! { __view(__cx, __parts, #ident); });
    }
}
