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
/// The hoist phase evaluates every expression of the view in source order
/// and binds the results to fresh identifiers. The burst phase pushes the
/// view's instruction block in one synchronous burst that only reads the
/// hoisted bindings.
///
/// A scope that renders a component, or fills a node position, resolves
/// its content by being polled: the expansion is the value expression of
/// the template's `JoinView`. Every dynamic node position is registered as
/// a *unit*, an inert view driven concurrently with the other units by the
/// join, and the burst phase becomes the join's burst closure, splicing the
/// contents the join resolved. After the burst builds the template's
/// content, the join keeps streaming the units' swaps.
///
/// A scope that renders no component and fills no node position builds
/// synchronously: the hoist phase runs and the burst pushes the block right
/// where the scope is evaluated, with control flow splicing the handles of
/// blocks built the same way. Nothing is polled, boxed, or captured.
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
    /// The units nest as `JoinUnit` pairs terminated by `()`, and their
    /// contents come back in the same nested shape, destructured into the
    /// `__view` identifiers the burst reads. The closure takes ownership of
    /// the hoisted bindings it reads, so the view owns everything it
    /// renders. What the hoisted bindings borrow stays alive for the whole
    /// block, so `tail` can consume the view where those borrows are still
    /// valid.
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
