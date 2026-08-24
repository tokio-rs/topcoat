use proc_macro2::{Span, TokenStream};
use quote::{TokenStreamExt, format_ident, quote, quote_spanned};
use syn::Ident;
use topcoat_core_grammar::paths::{topcoat_error, topcoat_view};

/// HIR nodes that emit themselves into the two phases of an [`Emitter`].
pub(crate) trait Emit {
    fn emit(&self, emitter: &mut Emitter);
}

/// The tuple arity [`Join`](topcoat_view::internal::Join) is implemented up
/// to; scopes with more units fall back to a boxed `Vec` join.
const MAX_TUPLE_UNITS: usize = 12;

/// Collects the two phases a view template expands to: the body of the
/// template's `ViewStream`.
///
/// The hoist phase evaluates every expression of the view in source order
/// and binds the results to fresh identifiers. Every dynamic node position —
/// a plain interpolation, a component, a control-flow construct — is
/// registered as a *unit*: its value is driven concurrently with the other
/// units through a `Join`, which resolves each position's rendered content.
/// The burst phase then pushes the view's instruction block in one
/// synchronous burst that only reads the hoisted bindings and the joined
/// content. Keeping every `await` out of the burst phase is what lets the
/// block land contiguously in the shared view buffer.
///
/// After the burst yields the template's content chunk, the join's remaining
/// chunks — live updates targeting their own positions — are forwarded
/// through the stream.
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
            if let ::core::option::Option::Some(__unit_view) = #view {
                __b.view(__unit_view);
            }
        });
    }

    /// Returns the statements joining the registered units: pinning each
    /// unit's future, resolving every position's content, and binding the
    /// content to the `__view` identifiers the burst reads.
    ///
    /// Up to [`MAX_TUPLE_UNITS`] units join as a tuple, allocation-free;
    /// beyond that they fall back to a boxed `Vec` join.
    fn join(&self) -> TokenStream {
        let view_idents = (0..self.units.len()).map(|i| format_ident!("__view{i}"));
        if self.units.len() <= MAX_TUPLE_UNITS {
            let pins = self.units.iter().enumerate().map(|(i, ident)| {
                let unit = format_ident!("__unit{i}");
                quote! {
                    let #unit = ::core::pin::pin!(
                        #topcoat_view::internal::unit_future(#ident, __cx)
                    );
                }
            });
            let units = (0..self.units.len()).map(|i| {
                let unit = format_ident!("__unit{i}");
                quote! { #topcoat_view::internal::Unit::new(#unit) }
            });
            quote! {
                #(#pins)*
                let mut __join = #topcoat_view::internal::Join::new((
                    #(#units,)*
                ));
                let (#(#view_idents,)*) = __join.first().await?;
            }
        } else {
            let pushes = self.units.iter().map(|ident| {
                quote! {
                    __units.push(#topcoat_view::internal::Unit::new(::std::boxed::Box::pin(
                        #topcoat_view::internal::unit_future(#ident, __cx),
                    )));
                }
            });
            quote! {
                let mut __units = ::std::vec::Vec::new();
                #(#pushes)*
                let mut __join = #topcoat_view::internal::Join::new(__units);
                let mut __view_contents = __join.first().await?.into_iter();
                #(let #view_idents = __view_contents.next().unwrap();)*
            }
        }
    }

    /// Returns the body of the template's `ViewStream`: the hoist phase, the
    /// join of the registered units, the burst that builds the instruction
    /// block and yields it as the content chunk, and the forwarding of the
    /// join's remaining chunks.
    pub(super) fn finish(self) -> TokenStream {
        let block = {
            let burst = &self.burst;
            quote! {
                let __view = #topcoat_view::internal::block(__cx, |__b| {
                    #burst
                });
                #topcoat_view::internal::emit_content(__view).await;
            }
        };
        let hoist = &self.hoist;
        if self.units.is_empty() {
            quote! {
                #hoist
                #block
                ::core::result::Result::<_, #topcoat_error::Error>::Ok(())
            }
        } else {
            let join = self.join();
            quote! {
                #hoist
                #join
                #block
                #topcoat_view::internal::forward(__join).await;
                ::core::result::Result::<_, #topcoat_error::Error>::Ok(())
            }
        }
    }
}
