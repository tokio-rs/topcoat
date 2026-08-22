use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote, quote_spanned};
use syn::{Path, spanned::Spanned};
use topcoat_core_grammar::paths::{topcoat_error, topcoat_view};

use crate::view::{
    NamedArg,
    hir::{
        Bindings, Scope,
        emit::{Emit, Emitter},
    },
};

/// A component invocation, emitted through the props builder.
pub(crate) struct Component {
    pub path: Path,
    pub named_args: Vec<NamedArg>,
    /// The `key:` argument keying the invocation's identity, if any.
    pub key: Option<NamedArg>,
    /// Numbers this invocation site within the expansion, telling sites
    /// apart when their spans collapse to a single location.
    pub ordinal: u32,
    /// Whether the invocation sits in a `for` body, so the site repeats.
    pub repeats: bool,
    pub children: Option<Scope>,
    pub span: Span,
}

impl Component {
    /// Returns the site key expression naming this invocation site.
    ///
    /// `file!`, `line!`, and `column!` are spanned onto the component path,
    /// so they resolve to the invocation's own location in source; the
    /// ordinal tells sites apart when a macro-generated body collapses
    /// their spans.
    fn site(&self) -> TokenStream {
        let ordinal = self.ordinal;
        quote_spanned! {self.path.span()=>
            const {
                #topcoat_view::identity::SiteKey::new(
                    ::core::file!(),
                    ::core::line!(),
                    ::core::column!(),
                    #ordinal,
                )
            }
        }
    }

    /// Returns the expression naming this invocation in the ambiguity
    /// error: the component path and its location in source.
    fn label(&self) -> TokenStream {
        let name = format!(
            "`{}`",
            self.path.to_token_stream().to_string().replace(' ', ""),
        );
        quote_spanned! {self.path.span()=>
            ::core::concat!(
                #name,
                " at ",
                ::core::file!(),
                ":",
                ::core::line!(),
                ":",
                ::core::column!(),
            )
        }
    }

    /// Wraps `future` to install this invocation's identity around every
    /// poll, deriving it from the identity installed at construction.
    ///
    /// A `key:` argument mixes its value into the identity, telling
    /// repetitions of the site apart. The value passes through the `key`
    /// method of a throwaway props builder, which hands it back through its
    /// callback: a real method call gives `key:` the completion, hover, and
    /// rename support of a prop, while the props of `future` stay untouched
    /// and carry nothing extra. Without a key, an invocation that repeats
    /// derives an ambiguous identity naming this invocation, so consuming
    /// the identity, or any identity below it, errors with a pointer to the
    /// missing `key`.
    fn identity_future(&self, future: &TokenStream) -> TokenStream {
        let span = self.path.span();
        let site = self.site();
        match (&self.key, self.repeats) {
            (Some(arg), _) => {
                let path = &self.path;
                let ident = &arg.ident;
                let value = &arg.value;
                quote_spanned! {span=> {
                    use #topcoat_view::Component;
                    let mut __key = ::core::option::Option::None;
                    #path::props_builder()
                        .#ident(#value, |__value| __key = ::core::option::Option::Some(__value));
                    #topcoat_view::identity::IdentityFuture::keyed(#site, __key.unwrap(), #future)
                }}
            }
            (None, true) => {
                let label = self.label();
                quote_spanned! {span=>
                    #topcoat_view::identity::IdentityFuture::ambiguous(#site, #label, #future)
                }
            }
            (None, false) => quote_spanned! {span=>
                #topcoat_view::identity::IdentityFuture::child(#site, #future)
            },
        }
    }
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
            let child = scope.emit_view();
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
    /// memory instead. The future joins the component's render with the
    /// children and redirects the slot to their view once both resolve, so a
    /// component renders concurrently with its own children at any nesting
    /// depth.
    fn render_future_with_async_children(&self, children: &Scope) -> TokenStream {
        let Self {
            path, named_args, ..
        } = self;

        let setters = named_args.iter().map(|arg| {
            let ident = &arg.ident;
            let value = &arg.value;
            quote! { .#ident(#value) }
        });
        let child = children.emit_future(&Bindings::empty());

        quote_spanned! {path.span()=> {
            async {
                use #topcoat_view::Component;
                let (__placeholder, __slot) = #topcoat_view::internal::reserve();
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
                let (__rendered, __child) =
                    #topcoat_view::internal::try_join!(__render, __child)?;
                __slot.fill(__child);
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
        let future = self.identity_future(&future);

        emitter.hoist_future(span, &ident, &future);
        emitter.burst(quote_spanned! {span=> __b.view(#ident); });
    }
}
