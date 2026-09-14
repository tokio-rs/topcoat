use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Expr, Pat};
use topcoat_core_grammar::paths::{topcoat_context, topcoat_core, topcoat_view};

use crate::view::hir::{
    Bindings, Scope,
    emit::{Emit, Emitter},
};

/// A `for` loop whose body is lowered into a nested scope.
pub(crate) struct ForLoop {
    pub pat: Pat,
    pub expr: Box<Expr>,
    pub key: Option<Box<Expr>>,
    pub ordinal: u32,
    pub body: Scope,
}

impl ForLoop {
    /// Derives this iteration's identity from the enclosing context.
    fn identity(&self) -> TokenStream {
        let ordinal = self.ordinal;
        let site = quote! {
            const {
                #topcoat_core::identity::SiteKey::new(
                    ::core::file!(),
                    ::core::line!(),
                    ::core::column!(),
                    #ordinal,
                )
            }
        };
        if let Some(key) = &self.key {
            quote! {
                #topcoat_context::identity_raw(__cx).keyed_child(#site, &(#key))
            }
        } else {
            quote! {
                #topcoat_context::identity_raw(__cx).ambiguous_child(
                    #site,
                    ::core::concat!(
                        "`for` loop at ",
                        ::core::file!(), ":", ::core::line!(), ":", ::core::column!(),
                    ),
                )
            }
        }
    }
}

impl Emit for ForLoop {
    fn emit(&self, emitter: &mut Emitter) {
        let ident = emitter.fresh_ident();
        let Self {
            pat, expr, body, ..
        } = self;
        let identity = self.identity();
        let context = quote! {
            #topcoat_context::with_identity(__cx.clone(), #identity)
        };

        if body.is_async() {
            // Each iteration owns its bindings and context until its view
            // finishes. LoopView drives them concurrently in source order.
            let inner = body.emit_driven();
            let body = Bindings::of_pattern(pat).emit_capture(quote! {
                let __cx = &#context;
                #inner
            });
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
            // Unkeyed iterations share one ambiguous context. Keep the
            // iterator expression in the enclosing context's scope.
            let shared_context = self.key.is_none().then(|| {
                quote! {
                    let __loop_cx = #context;
                }
            });
            let iteration_context = if self.key.is_some() {
                quote! { &#context }
            } else {
                quote! { &__loop_cx }
            };
            emitter.hoist(quote! {
                let #ident = {
                    #shared_context
                    let mut __views = ::std::vec::Vec::new();
                    for #pat in #expr {
                        __views.push({
                            let __cx = #iteration_context;
                            #body
                        });
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
