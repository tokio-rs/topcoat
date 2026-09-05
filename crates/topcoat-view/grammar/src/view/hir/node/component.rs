use proc_macro2::{Ident, Span, TokenStream};
use quote::{ToTokens, quote, quote_spanned};
use syn::{Path, spanned::Spanned};
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
    /// The `key:` argument keying the invocation's identity, if any.
    pub key: Option<NamedArg>,
    /// Numbers this invocation site within the expansion. Every site key in
    /// one expansion resolves to the `view!` invocation's location, so the
    /// ordinal is what tells the sites apart.
    pub ordinal: u32,
    /// Whether the invocation sits in a `for` body, so the site repeats.
    pub repeats: bool,
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
    /// error: the component path and the `view!` invocation's location in
    /// source.
    fn label(&self) -> TokenStream {
        let name = format!(
            "`{}`",
            self.path.to_token_stream().to_string().replace(' ', ""),
        );
        quote! {
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

    /// Emits the guard entering this invocation's identity, derived from the
    /// identity installed where the invocation sits.
    ///
    /// A `key:` argument mixes its value into the identity, telling
    /// repetitions of the site apart. The value passes through the `key`
    /// method of a throwaway props builder, which hands it back through its
    /// callback: a real method call gives `key:` the completion, hover, and
    /// rename support of a prop, while the invocation's own props stay
    /// untouched and carry nothing extra. Without a key, an invocation that
    /// repeats derives an ambiguous identity naming this invocation, so
    /// consuming the identity, or any identity below it, errors with a
    /// pointer to the missing `key`.
    fn identity_guard(&self) -> TokenStream {
        let site = self.site();
        match (&self.key, self.repeats) {
            (Some(arg), _) => {
                let path = &self.path;
                let props_builder = self.diagnostic_ident("props_builder");
                let ident = &arg.ident;
                let value = &arg.value;
                quote! {{
                    use #topcoat_view::Component;
                    let mut __key = ::core::option::Option::None;
                    #path::#props_builder()
                        .#ident(#value, |__value| __key = ::core::option::Option::Some(__value));
                    #topcoat_view::identity::IdentityGuard::enter_keyed(#site, __key.unwrap())
                }}
            }
            (None, true) => {
                let label = self.label();
                quote! {
                    #topcoat_view::identity::IdentityGuard::enter_ambiguous(#site, #label)
                }
            }
            (None, false) => quote! {
                #topcoat_view::identity::IdentityGuard::enter(#site)
            },
        }
    }
    /// Returns the expression yielding this component's render future, with
    /// the props evaluated eagerly.
    ///
    /// The invocation's child nodes pass as a [`Child`]: the component
    /// decides where they render by interpolating the value into its own
    /// template, which drives them concurrently with the rest of it.
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
            let child = scope.emit_inert();
            quote! { .child(#topcoat_view::Child::new(#child)) }
        });

        let props_builder = self.diagnostic_ident("props_builder");
        let build = self.diagnostic_ident("build");
        let render = self.diagnostic_ident("render");
        quote! {{
            use #topcoat_view::Component;
            let props = #path::#props_builder()#(#setters)*#child.#build();
            // The marker is built via `Default` so the same construction
            // works for both unit-struct and generic (`PhantomData`) markers.
            #[allow(clippy::default_constructed_unit_structs)]
            Component::#render(
                #path::default(),
                __cx,
                props,
            )
        }}
    }
}

impl Emit for Component {
    fn emit(&self, emitter: &mut Emitter) {
        let ident = emitter.fresh_ident();
        let span = self.span;

        // The props are built under the invocation's identity, so a child
        // view carries it too. Once the guard is gone, the body future and
        // the view it resolves to poll at that same identity, so the
        // invocations in the body's template derive from it as well. The
        // same polls collect what the body hoists, so hoisted parts land
        // ahead of this invocation's content.
        let guard = self.identity_guard();
        let future = self.render_future();

        emitter.hoist(quote_spanned! {span=>
            let #ident = {
                let __guard = #guard;
                let __identity = #topcoat_view::identity::IdentityGuard::identity(&__guard);
                let __future = #future;
                ::core::mem::drop(__guard);
                #topcoat_view::identity::IdentityView::new(
                    __identity,
                    #topcoat_view::HoistView::new(
                        #topcoat_view::internal::ThenView::new(__future),
                    ),
                )
            };
        });
        emitter.unit(span, &ident);
    }
}
