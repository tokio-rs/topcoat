use proc_macro2::Span;
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

        if body.is_async() {
            // A body that renders components yields one future per
            // iteration; joining them renders all iterations concurrently.
            // The future owns its iteration's bindings, so it outlives the
            // iteration that produced it.
            let body = body.emit_owned_future();
            emitter.hoist_result_future(
                Span::call_site(),
                &ident,
                &quote! {{
                    let mut __futures = ::std::vec::Vec::new();
                    for #pat in #expr {
                        __futures.push(#body);
                    }
                    __try_join_all(__futures)
                }},
            );
        } else {
            // The hoist phase builds one view per iteration; the emit phase
            // splices them into the enclosing block in iteration order.
            let body = body.emit_expr();
            emitter.hoist(quote! {
                let #ident = {
                    let mut __views = ::std::vec::Vec::new();
                    for #pat in #expr {
                        __views.push(#body);
                    }
                    __views
                };
            });
        }
        emitter.emit(quote! {
            for __loop_view in #ident {
                __view(__cx, __parts, __loop_view);
            }
        });
    }
}
