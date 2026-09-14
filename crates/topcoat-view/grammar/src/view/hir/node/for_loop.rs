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

        if body.is_async() {
            // The iterations become one `LoopView`, driven as one unit of
            // the enclosing template, so all iterations render concurrently
            // and splice in iteration order. Each iteration's view is built
            // inside the iteration and takes the pattern's bindings with
            // it. The views share one type, and pinning each on the heap
            // lets the loop hold them in a plain `Vec`.
            let body = body.emit_captured_with(&Bindings::of_pattern(pat), |scope| {
                let identity = self.identity();
                let inner = scope.emit_inner(|view| {
                    quote! {
                        #topcoat_view::internal::MoveView::drive(#view).await
                    }
                });
                quote! {
                    let __cx = &#topcoat_context::with_identity(__cx.clone(), #identity);
                    #inner
                }
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
            let identity = self.identity();
            let body = quote! {{
                let __cx = &#topcoat_context::with_identity(__cx.clone(), #identity);
                #body
            }};
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
