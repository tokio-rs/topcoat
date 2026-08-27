use proc_macro2::{Span, TokenStream};
use quote::{TokenStreamExt, format_ident, quote, quote_spanned};
use syn::Ident;
use topcoat_core_grammar::paths::topcoat_view;

/// HIR nodes that emit themselves into the two phases of an [`Emitter`].
pub(crate) trait Emit {
    fn emit(&self, emitter: &mut Emitter);
}

/// Collects the two phases a view template expands to: the value expression
/// of the template's `JoinView`.
///
/// The hoist phase evaluates every expression of the view in source order
/// and binds the results to fresh identifiers. Every dynamic node position —
/// a plain interpolation, a component, a control-flow construct — is
/// registered as a *unit*: an inert view driven concurrently with the other
/// units by the `JoinView`, which resolves each position's content. The
/// burst phase becomes the join's burst closure: it pushes the view's
/// instruction block in one synchronous burst that only reads the hoisted
/// bindings and the resolved contents.
///
/// After the burst builds the template's content, the join keeps streaming
/// the units' swaps — live updates targeting their own regions.
pub(crate) struct Emitter {
    hoist: TokenStream,
    burst: TokenStream,
    counter: u32,
    /// The hoisted bindings joined as units, in position order.
    units: Vec<Ident>,
}

impl Emitter {
    pub(super) fn new() -> Self {
        Self {
            hoist: TokenStream::new(),
            burst: TokenStream::new(),
            counter: 0,
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
    pub(super) fn unit(&mut self, span: Span, ident: &Ident) {
        let view = format_ident!("__view{}", self.units.len());
        self.units.push(ident.clone());
        self.burst(quote_spanned! {span=>
            __b.view(#view);
        });
    }

    /// Returns a block that runs the hoist phase, builds the template's
    /// `JoinView` against the ambient `__cx` context and `__buf` buffer,
    /// and ends with `tail` applied to the join expression.
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
                __buf,
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
}
