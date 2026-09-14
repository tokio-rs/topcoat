use proc_macro2::{Span, TokenStream};
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
    pub key: Option<Expr>,
    pub ordinal: u32,
    pub body: Scope,
}

impl ForLoop {
    /// Builds and polls the iteration body under its identity.
    fn identity_body(&self, body: TokenStream, is_async: bool) -> TokenStream {
        let ordinal = self.ordinal;
        let site = quote! {
            const {
                #topcoat_view::identity::SiteKey::new(
                    ::core::file!(),
                    ::core::line!(),
                    ::core::column!(),
                    #ordinal,
                )
            }
        };
        let guard = match &self.key {
            Some(key) => quote! {
                #topcoat_view::identity::IdentityGuard::enter_keyed(#site, &(#key))
            },
            None => quote! {
                #topcoat_view::identity::IdentityGuard::enter_ambiguous(
                    #site,
                    ::core::concat!(
                        "`for` loop at ",
                        ::core::file!(), ":", ::core::line!(), ":", ::core::column!(),
                    ),
                )
            },
        };
        let view = if is_async {
            quote! { #topcoat_view::identity::IdentityView::new(__identity, __view) }
        } else {
            quote! { __view }
        };
        quote! {{
            let __guard = #guard;
            let __identity = #topcoat_view::identity::IdentityGuard::identity(&__guard);
            let __view = #body;
            ::core::mem::drop(__guard);
            #view
        }}
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
            let body = body.emit_captured(&Bindings::of_pattern(pat));
            let body = self.identity_body(body, true);
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
            let body = self.identity_body(body, false);
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
