use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::{Path, spanned::Spanned};
use topcoat_core_grammar::paths::{topcoat_context, topcoat_core, topcoat_view};

use crate::view::{
    NamedArg,
    hir::{
        Scope,
        emit::{Emit, Emitter},
    },
};

/// A component invocation, emitted through the props builder.
pub(crate) struct Component {
    pub path: Path,
    pub named_args: Vec<NamedArg>,
    /// Numbers this invocation site within the expansion. Every site key in
    /// one expansion resolves to the `view!` invocation's location, so the
    /// ordinal is what tells the sites apart.
    pub ordinal: u32,
    pub children: Option<Scope>,
    pub span: Span,
}

impl Component {
    /// Returns `name` as an ident spanned onto the component path, so the
    /// error for a missing prop or a path that is not a component points at
    /// the invocation.
    ///
    /// These idents are the only generated tokens carrying the path's span.
    /// Anything spanned onto the path shows up when the editor hovers the
    /// component name, so the rest of the emission uses call-site spans to
    /// keep the hover down to the component and its props methods.
    fn diagnostic_ident(&self, name: &str) -> Ident {
        Ident::new(name, self.path.span())
    }

    /// Returns the site key expression naming this invocation site.
    ///
    /// `file!`, `line!`, and `column!` carry call-site spans, so they
    /// resolve to the `view!` invocation's location; the ordinal tells the
    /// sites within one expansion apart.
    fn site(&self) -> TokenStream {
        let ordinal = self.ordinal;
        quote! {
            const {
                #topcoat_core::identity::SiteKey::new(
                    ::core::file!(),
                    ::core::line!(),
                    ::core::column!(),
                    #ordinal,
                )
            }
        }
    }

}

impl Emit for Component {
    fn emit(&self, emitter: &mut Emitter) {
        let ident = emitter.fresh_ident();
        let span = self.span;

        let site = self.site();
        let path = &self.path;
        let props_builder = self.diagnostic_ident("props_builder");
        let build = self.diagnostic_ident("build");
        let render = self.diagnostic_ident("render");
        let setters = self.named_args.iter().map(|arg| {
            let ident = &arg.ident;
            let value = &arg.value;
            quote! { .#ident(#value) }
        });
        let child = self.children.as_ref().map(|scope| {
            let child = scope.emit_inert();
            quote! { .child(#topcoat_view::Child::new(#child)) }
        });

        // Evaluate named arguments first. Build child content inside the
        // future so it borrows the context kept alive through rendering.
        emitter.hoist(quote_spanned! {span=>
            let #ident = {
                use #topcoat_view::Component;
                let __props = #path::#props_builder()#(#setters)*;
                let __cx = #topcoat_context::with_identity(
                    __cx.clone(), #topcoat_context::identity_raw(__cx).child(#site),
                );
                let __captured = #topcoat_view::internal::Capture((__cx, __props));
                #topcoat_view::HoistView::new(
                    #topcoat_view::internal::MoveView::new(async {
                        let (__cx, __props) = __captured.take();
                        let __cx = &__cx;
                        let __props = __props #child.#build();
                        #[allow(clippy::default_constructed_unit_structs)]
                        let __view = Component::#render(#path::default(), __cx, __props).await?;
                        #topcoat_view::internal::MoveView::drive(__view).await
                    }),
                )
            };
        });
        emitter.unit(span, &ident);
    }
}
