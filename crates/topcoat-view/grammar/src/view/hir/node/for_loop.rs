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
            // The iterations become one `LoopView`, driven as one unit of
            // the enclosing template, so all iterations render concurrently
            // and splice in iteration order. Each iteration's view is built
            // inside the iteration and takes the pattern's bindings with
            // it. The views share one type, and pinning each on the heap
            // lets the loop hold them in a plain `Vec`.
            let body = body.emit_captured(&Bindings::of_pattern(pat));
            emitter.hoist(quote! {
                let #ident = {
                    let mut __iterations = ::std::vec::Vec::new();
                    for #pat in #expr {
                        __iterations.push(::std::boxed::Box::pin(#body));
                    }
                    #topcoat_view::internal::LoopView::new(__iterations)
                };
            });
            emitter.unit(Span::call_site(), &ident);
        } else {
            // Each iteration builds its block right inside the iteration,
            // where the pattern's bindings are alive; the burst splices the
            // handles in iteration order.
            let body = body.emit_block();
            emitter.hoist(quote! {
                let #ident = {
                    let mut __views = ::std::vec::Vec::new();
                    for #pat in #expr {
                        __views.push(#body);
                    }
                    __views
                };
            });
            emitter.burst(quote! {
                for __view in #ident {
                    __b.view(__view);
                }
            });
        }
    }
}
