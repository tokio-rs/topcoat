use proc_macro2::Span;
use quote::{quote, quote_spanned};
use syn::Path;
use topcoat_core_grammar::paths::topcoat_view;

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
    pub children: Option<Scope>,
    pub span: Span,
}

impl Emit for Component {
    fn emit(&self, emitter: &mut Emitter) {
        let ident = emitter.fresh_ident();
        let Self {
            path,
            named_args,
            children,
            span,
        } = self;

        let setters = named_args.iter().map(|arg| {
            let ident = &arg.ident;
            let value = &arg.value;
            quote! { .#ident(#value) }
        });
        let child = children.as_ref().map(|scope| {
            let child = scope.emit_expr();
            quote_spanned! {*span=> .child(#child) }
        });

        emitter.hoist(quote_spanned! {*span=>
            let #ident = {
                use #topcoat_view::Component;
                let props = #path::props_builder()#(#setters)*#child.build();
                // The marker is built via `Default` so the same construction
                // works for both unit-struct and generic (`PhantomData`) markers.
                #[allow(clippy::default_constructed_unit_structs)]
                Component::render(
                    #path::default(),
                    __cx,
                    props,
                ).await?
            };
        });
        emitter.emit(quote_spanned! {*span=> __view(__cx, __parts, #ident); });
    }
}
