use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::Expr;
use topcoat_core_grammar::paths::topcoat_view;

use crate::view::hir::{
    Component, MatchArm,
    emit::{Emit, Emitter},
};

/// A `live` construct: view control flow that consumes a reactive expression
/// and re-renders its arms in place when the state changes.
///
/// `live match` lowers here directly; `live if let` lowers as a match with a
/// wildcard arm for the missing state.
pub(crate) struct LiveNode {
    /// Numbers this node within the expansion; the frame's prelude
    /// registers its slot and ticket under this ordinal.
    pub ordinal: u32,
    pub scrutinee: LiveScrutinee,
    pub arms: Vec<MatchArm>,
    pub span: Span,
}

/// The reactive expression a [`LiveNode`] consumes.
pub(crate) enum LiveScrutinee {
    /// A `defer(future)` call; the expansion supplies the context argument.
    Defer(Box<Expr>),
    /// A component invocation, adopted so its states arrive at the node.
    Component(Box<Component>),
    /// Any other expression, already a reactive value.
    Expr(Box<Expr>),
}

impl LiveNode {
    /// Returns the expression evaluating to the node's reactive value, in
    /// hoist position.
    fn reactive(&self, span: Span) -> TokenStream {
        match &self.scrutinee {
            LiveScrutinee::Defer(future) => quote_spanned! {span=>
                #topcoat_view::live::defer(__cx, #future)
            },
            LiveScrutinee::Component(component) => component.emit_adopt(),
            LiveScrutinee::Expr(expr) => quote_spanned! {span=> #expr },
        }
    }
}

impl Emit for LiveNode {
    fn emit(&self, emitter: &mut Emitter) {
        let span = self.span;
        if !emitter.live() {
            emitter.hoist(quote_spanned! {span=>
                ::core::compile_error!(
                    "`live` needs a live render: use it inside a `#[component]`, \
                     `#[page]`, or `#[layout]` function",
                );
            });
            return;
        }

        let view = format_ident!("__node{}", self.ordinal);
        let slot = format_ident!("__node_slot{}", self.ordinal);
        let ticket = format_ident!("__node_ticket{}", self.ordinal);
        let state = format_ident!("__node_state{}", self.ordinal);
        let reactive = self.reactive(span);

        // Each arm body is a full frame of its own: one run of the arms
        // renders the state into the node's slot and keeps driving whatever
        // the arm started.
        let delivery = quote! {
            #slot.refill(__view);
            #ticket.hand_over();
        };
        let arms = self.arms.iter().map(|arm| {
            let pat = &arm.pat;
            let guard = arm.guard.as_ref().map(|guard| quote! { if #guard });
            let body = arm.body.emit_frame_live(&TokenStream::new(), &delivery);
            quote! { #pat #guard => #body }
        });

        // The node future: await a state, run the matching arm's frame, then
        // race the shown arm's remaining work against the next state. The
        // block is a plain `async`, so the arms borrow the surrounding
        // frame's locals; the slot, the ticket, and the reactive value come
        // in through the prelude and the `Own` wrapper, so the entry holds
        // no borrow of anything the frame's set outlives.
        emitter.hoist(quote_spanned! {span=>
            let #state = #topcoat_view::internal::Own(#reactive);
            __refresh.push(async {
                let __own = #state;
                let mut __reactive = __own.0;
                // The state is wrapped so the arm takes it by value even
                // when its type is `Copy`, which a plain `async` block would
                // otherwise capture by reference across the loop.
                let mut __state =
                    match #topcoat_view::internal::Reactive::next_state(&mut __reactive).await {
                        ::core::option::Option::Some(__state) => {
                            #topcoat_view::internal::Own(__state)
                        }
                        ::core::option::Option::None => {
                            return ::core::result::Result::Ok(());
                        }
                    };
                loop {
                    let mut __arm = ::core::pin::pin!(async {
                        let __own = __state;
                        match __own.0 {
                            #(#arms,)*
                        }
                    });
                    let mut __next = ::core::pin::pin!(
                        #topcoat_view::internal::Reactive::next_state(&mut __reactive)
                    );
                    match #topcoat_view::internal::race_node(__arm.as_mut(), __next.as_mut()).await
                    {
                        // A new state: dropping the arm at the end of this
                        // pass cancels the replaced subtree's work.
                        #topcoat_view::internal::Raced::State(
                            ::core::option::Option::Some(__new_state),
                        ) => {
                            __state = #topcoat_view::internal::Own(__new_state);
                        }
                        // Retired: finish the shown arm's remaining work.
                        #topcoat_view::internal::Raced::State(::core::option::Option::None) => {
                            return __arm.await;
                        }
                        // The arm finished first: keep listening.
                        #topcoat_view::internal::Raced::ArmDone(__result) => {
                            __result?;
                            match __next.await {
                                ::core::option::Option::Some(__new_state) => {
                                    __state = #topcoat_view::internal::Own(__new_state);
                                }
                                ::core::option::Option::None => {
                                    return ::core::result::Result::Ok(());
                                }
                            }
                        }
                    }
                }
            });
        });
        emitter.burst(quote_spanned! {span=> __b.view(#view); });
    }
}
