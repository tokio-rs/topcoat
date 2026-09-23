use proc_macro2::{Span, TokenStream};
use quote::{TokenStreamExt, format_ident, quote, quote_spanned};
use syn::Ident;
use topcoat_core_grammar::paths::topcoat_view;

/// HIR nodes that emit themselves into the two phases of an [`Emitter`].
pub(crate) trait Emit {
    fn emit(&self, emitter: &mut Emitter);
}

/// Collects the two phases a view template expands to.
///
/// The hoist phase evaluates expressions in source order and binds their
/// results to fresh identifiers. The burst phase writes an instruction
/// block synchronously using those bindings.
///
/// Scopes with dynamic nodes use a `JoinView`. It polls each node as a
/// unit, then runs the burst with the resolved content. Later updates
/// stream through the join.
///
/// Scopes without dynamic nodes build synchronously. Their hoist and burst
/// phases run together where the scope is evaluated.
pub(crate) struct Emitter {
    hoist: TokenStream,
    burst: TokenStream,
    counter: u32,
    /// Whether the scope builds synchronously instead of as a join.
    sync: bool,
    /// The hoisted bindings joined as units, in position order.
    units: Vec<Ident>,
}

impl Emitter {
    pub(super) fn new(sync: bool) -> Self {
        Self {
            hoist: TokenStream::new(),
            burst: TokenStream::new(),
            counter: 0,
            sync,
            units: Vec::new(),
        }
    }

    /// Returns a fresh identifier for a hoisted binding.
    pub(super) fn fresh_ident(&mut self) -> Ident {
        let ident = format_ident!("__expr{}", self.counter);
        self.counter += 1;
        ident
    }

    /// Appends statements to the hoist phase.
    pub(super) fn hoist(&mut self, tokens: TokenStream) {
        self.hoist.append_all(tokens);
    }

    /// Appends pushes on the `__b` builder to the burst phase.
    pub(super) fn burst(&mut self, tokens: TokenStream) {
        self.burst.append_all(tokens);
    }

    /// Registers the hoisted binding `ident` as a joined unit and splices
    /// the content the join resolves for it into the burst.
    ///
    /// # Panics
    ///
    /// Panics if the scope builds synchronously: a synchronous scope has no
    /// join to drive units.
    pub(super) fn unit(&mut self, span: Span, ident: &Ident) {
        assert!(!self.sync, "a synchronous scope joins no units");
        let view = format_ident!("__view{}", self.units.len());
        self.units.push(ident.clone());
        self.burst(quote_spanned! {span=>
            __b.view(#view);
        });
    }

    /// Returns a block that runs the hoist phase, builds the template's
    /// `JoinView` against the ambient `__cx` context, and ends with `tail`
    /// applied to the join expression.
    ///
    /// Units form a nested list ending in `()`. Their contents use the same
    /// structure and bind to the `__view` identifiers read by the burst.
    /// The burst owns its captured bindings. `tail` can consume the view
    /// before anything borrowed by those bindings leaves scope.
    pub(super) fn finish(self, tail: impl FnOnce(TokenStream) -> TokenStream) -> TokenStream {
        let units = self.units.iter().rev().fold(quote! { () }, |rest, ident| {
            quote! { #topcoat_view::internal::JoinUnit::new(#ident, #rest) }
        });
        let contents = (0..self.units.len())
            .rev()
            .fold(quote! { () }, |rest, index| {
                let view = format_ident!("__view{index}");
                quote! { (#view, #rest) }
            });
        let hoist = &self.hoist;
        let burst = &self.burst;
        let tail = tail(quote! {
            #topcoat_view::internal::JoinView::new(
                __cx,
                #units,
                move |__b, #contents| {
                    #burst
                },
            )
        });
        quote! {{
            #hoist
            #tail
        }}
    }

    /// Returns a block that runs the hoist phase and then builds the
    /// scope's instruction block in one burst against the ambient `__cx`
    /// context, yielding the handle to the block.
    ///
    /// The block lands in the buffer of the build right where the scope is
    /// evaluated, so the burst may read what the iteration or branch around
    /// it binds.
    pub(super) fn finish_block(self) -> TokenStream {
        debug_assert!(self.sync, "a joined scope finishes as a join");
        let hoist = &self.hoist;
        let burst = &self.burst;
        quote! {{
            #hoist
            #topcoat_view::internal::Builder::block(__cx, |__b| {
                #burst
            })
        }}
    }
}
