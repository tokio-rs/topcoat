use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
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

impl Component {
    /// Returns the expression yielding this component's render future, with
    /// the props evaluated eagerly.
    fn render_future(&self) -> TokenStream {
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

        quote_spanned! {*span=> {
            use #topcoat_view::Component;
            let props = #path::props_builder()#(#setters)*#child.build();
            // The marker is built via `Default` so the same construction
            // works for both unit-struct and generic (`PhantomData`) markers.
            #[allow(clippy::default_constructed_unit_structs)]
            Component::render(
                #path::default(),
                __cx,
                props,
            )
        }}
    }

    /// Returns the expression yielding this component's render future when
    /// the children render components of their own.
    ///
    /// The named args and the children's future still evaluate eagerly in
    /// source order; the returned future joins in the children, builds the
    /// props, and renders. It captures only the evaluated args and the
    /// children's future, so it composes with the joins of the enclosing
    /// scope like a plain render future.
    fn render_future_with_async_children(&self, children: &Scope) -> TokenStream {
        let Self {
            path,
            named_args,
            span,
            ..
        } = self;

        let bindings = named_args.iter().enumerate().map(|(index, arg)| {
            let ident = format_ident!("__arg{index}");
            let value = &arg.value;
            quote! { let #ident = #value; }
        });
        let setters = named_args.iter().enumerate().map(|(index, arg)| {
            let ident = &arg.ident;
            let value = format_ident!("__arg{index}");
            quote! { .#ident(#value) }
        });
        let child = children.emit_future();

        quote_spanned! {*span=> {
            use #topcoat_view::Component;
            #(#bindings)*
            let __child = #child;
            async move {
                let __child = __child.await?;
                let props = #path::props_builder()#(#setters)*.child(__child).build();
                // The marker is built via `Default` so the same construction
                // works for both unit-struct and generic (`PhantomData`)
                // markers.
                #[allow(clippy::default_constructed_unit_structs)]
                Component::render(
                    #path::default(),
                    __cx,
                    props,
                ).await
            }
        }}
    }
}

impl Emit for Component {
    fn emit(&self, emitter: &mut Emitter) {
        let ident = emitter.fresh_ident();
        let span = self.span;

        // Children that render components of their own await while the props
        // build. With inline awaits that is fine; in a joined position the
        // awaits have to move into the render future instead.
        let future = match &self.children {
            Some(children) if !emitter.inline_await() && children.is_async() => {
                self.render_future_with_async_children(children)
            }
            _ => self.render_future(),
        };

        emitter.hoist_result_future(span, &ident, &future);
        emitter.emit(quote_spanned! {span=> __view(__cx, __parts, #ident); });
    }
}
