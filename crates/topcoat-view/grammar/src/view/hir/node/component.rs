use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::{Path, spanned::Spanned};
use topcoat_core_grammar::paths::{topcoat_error, topcoat_view};

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
            ..
        } = self;

        let setters = named_args.iter().map(|arg| {
            let ident = &arg.ident;
            let value = &arg.value;
            quote! { .#ident(#value) }
        });
        let child = children.as_ref().map(|scope| {
            let child = scope.emit_expr();
            quote! { .child(#child) }
        });

        quote_spanned! {path.span()=> {
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
    /// The children cannot resolve before the props build, so the props take
    /// a placeholder view holding a reserved slot in the scope's instruction
    /// memory instead. The returned future joins the component's render with
    /// a future that awaits the children and redirects the slot to their
    /// view, so a component renders concurrently with its own children at
    /// any nesting depth.
    fn render_future_with_async_children(&self, children: &Scope) -> TokenStream {
        let Self {
            path, named_args, ..
        } = self;

        let setters = named_args.iter().map(|arg| {
            let ident = &arg.ident;
            let value = &arg.value;
            quote! { .#ident(#value) }
        });
        let child = children.emit_future();

        quote_spanned! {path.span()=> {
            async {
                use #topcoat_view::Component;
                let (__placeholder, __slot) = __reserve_view();
                let props = #path::props_builder()#(#setters)*.child(__placeholder).build();
                // The marker is built via `Default` so the same construction
                // works for both unit-struct and generic (`PhantomData`) markers.
                #[allow(clippy::default_constructed_unit_structs)]
                let __render = Component::render(
                    #path::default(),
                    __cx,
                    props,
                );
                let __child = #child;

                let (__rendered, __child) = __try_join!(__render, __child)?;

                __fill_view(__slot, __child);
                
                ::core::result::Result::<_, #topcoat_error::Error>::Ok(__rendered)
            }
        }}
    }
}

impl Emit for Component {
    fn emit(&self, emitter: &mut Emitter) {
        let ident = emitter.fresh_ident();
        let span = self.span;

        // Children that render components of their own resolve through a
        // reserved slot, so the component overlaps with them instead of
        // awaiting them while the props build.
        let future = match &self.children {
            Some(children) if children.is_async() => {
                self.render_future_with_async_children(children)
            }
            _ => self.render_future(),
        };

        emitter.hoist_future(span, &ident, &future);
        emitter.emit(quote_spanned! {span=> __view(__cx, __parts, #ident); });
    }
}
