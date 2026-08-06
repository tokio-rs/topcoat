use quote::quote;
use syn::{Expr, Pat};

use crate::view::hir::{
    Scope,
    emit::{Emit, Emitter},
};

/// A `for` loop whose body is lowered into a nested scope.
pub(crate) struct ForLoop {
    pub pat: Pat,
    pub expr: Box<Expr>,
    pub body: Scope,
}

impl Emit for ForLoop {
    fn emit(&self, emitter: &mut Emitter) {
        let ident = emitter.fresh_ident();
        let Self { pat, expr, body } = self;
        let body = body.emit_expr();

        // The hoist phase builds one view per iteration; the emit phase
        // splices them into the enclosing block in iteration order.
        emitter.hoist(quote! {
            let #ident = {
                let mut __views = ::std::vec::Vec::new();
                for #pat in #expr {
                    __views.push(#body);
                }
                __views
            };
        });
        emitter.emit(quote! {
            for __loop_view in #ident {
                __view(__cx, __parts, __loop_view);
            }
        });
    }
}
