use proc_macro2::Span;
use quote::quote;
use syn::{Expr, Pat};
use topcoat_core_grammar::paths::topcoat_view;

use crate::view::hir::{
    Bindings, Scope,
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
            // Each future carries its iteration's pattern bindings, which
            // die with the iteration, and borrows the rest of its
            // environment, so iterations share outer values.
            let body = body.emit_future(&Bindings::of_pattern(pat));
            emitter.hoist_future(
                Span::call_site(),
                &ident,
                &quote! {{
                    let mut __futures = ::std::vec::Vec::new();
                    for #pat in #expr {
                        __futures.push(#body);
                    }
                    #topcoat_view::internal::try_join_all(__futures)
                }},
            );
        } else {
            // The hoist phase builds one view per iteration; the burst phase
            // splices them into the enclosing block in iteration order.
            let body = body.emit_view();
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
        emitter.burst(quote! {
            for __loop_view in #ident {
                __b.view(__loop_view);
            }
        });
    }
}
