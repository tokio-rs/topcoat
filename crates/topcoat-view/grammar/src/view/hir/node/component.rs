use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::Path;
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
    /// The span of the invocation's opening parenthesis, which every
    /// generated token carries apart from the path.
    ///
    /// Editors resolve each generated token mapped onto a source token, and
    /// also each one whose span contains it. A single-character span keeps
    /// errors such as a missing prop next to the component name while
    /// leaving go-to-definition on the name, and on anything inside the
    /// parentheses, with only that token's own definition.
    pub span: Span,
}

impl Component {
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
                let __props = #path::props_builder()#(#setters)*;
                let __cx = #topcoat_context::with_identity(
                    __cx.clone(), #topcoat_context::identity_raw(__cx).child(#site),
                );
                let __captured = #topcoat_view::internal::Capture((__cx, __props));
                #topcoat_view::HoistView::new(
                    #topcoat_view::internal::MoveView::new(async {
                        let (__cx, __props) = __captured.take();
                        let __cx = &__cx;
                        let __props = __props #child.build();
                        #[allow(clippy::default_constructed_unit_structs)]
                        let __view = Component::render(#path::default(), __cx, __props).await?;
                        #topcoat_view::internal::MoveView::drive(__view).await
                    }),
                )
            };
        });
        emitter.unit(span, &ident);
    }
}
