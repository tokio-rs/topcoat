use proc_macro2::Span;
use quote::quote;
use syn::{Expr, Pat};
use topcoat_core_grammar::paths::{topcoat_error, topcoat_view};

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

        if !body.is_async() {
            // The hoist phase builds one view per iteration; the burst phase
            // splices them into the enclosing block in iteration order.
            let body = if emitter.live() {
                body.emit_view_live()
            } else {
                body.emit_view()
            };
            emitter.hoist(quote! {
                let #ident = {
                    let mut __views = ::std::vec::Vec::new();
                    for #pat in #expr {
                        __views.push(#body);
                    }
                    __views
                };
            });
        } else if emitter.live() {
            // Each iteration is a frame of its own, filling a reserved slot,
            // and one pushed entry drives all the iteration futures: the
            // loop adds one entry and one `Vec` to the frame, not one entry
            // per item, and the first error wins.
            let delivery = quote! { __iteration_fill.fill(__view); };
            let body = body.emit_frame_live(&proc_macro2::TokenStream::new(), &delivery);
            emitter.hoist(quote! {
                let #ident = {
                    let mut __views = ::std::vec::Vec::new();
                    let mut __futures = ::std::vec::Vec::new();
                    for #pat in #expr {
                        let (__iteration_view, __iteration_fill) = __refresh.reserve_invoke();
                        __views.push(__iteration_view);
                        __futures.push(async move { #body });
                    }
                    __refresh.push(async move {
                        #topcoat_view::internal::try_join_all(__futures).await?;
                        ::core::result::Result::<(), #topcoat_error::Error>::Ok(())
                    });
                    __views
                };
            });
        } else {
            // A body that renders components yields one future per
            // iteration; joining them renders all iterations concurrently.
            // The future owns its iteration's bindings, so it outlives the
            // iteration that produced it.
            let body = body.emit_owned_future();
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
        }
        emitter.burst(quote! {
            for __loop_view in #ident {
                __b.view(__loop_view);
            }
        });
    }
}
