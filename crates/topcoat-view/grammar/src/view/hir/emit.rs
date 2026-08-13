use proc_macro2::{Span, TokenStream};
use quote::{TokenStreamExt, format_ident, quote, quote_spanned};
use syn::Ident;
use topcoat_core_grammar::paths::topcoat_view;

/// HIR nodes that emit themselves into the two phases of an [`Emitter`].
pub(crate) trait Emit {
    fn emit(&self, emitter: &mut Emitter);
}

/// How a scope's expansion runs.
///
/// The two modes are the two ways a `view!` executes. Inside a component
/// transform the expansion is live: component invocations register with the
/// frame's `RefreshSet` and reactive constructs compile to nodes. Outside
/// one there is no frame, so the expansion is blocking: every invocation is
/// driven to completion where it is awaited, and reactive constructs are
/// rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EmitMode {
    Blocking,
    Live,
}

/// Collects the two phases a view expression expands to.
///
/// The hoist phase evaluates every expression of the view in source order,
/// including building the views of nested control flow, and binds the results
/// to fresh identifiers. The burst phase then pushes the view's instruction
/// block in one synchronous burst that only reads those bindings. Keeping
/// every `await`, and every user expression that may build a view of its
/// own, out of the burst phase is what lets the block land contiguously in
/// the scope's shared view buffer.
///
/// In blocking mode component renders are futures. With inline awaits, the
/// hoist phase awaits each one where it is bound. Without them, the hoist
/// phase only binds the futures and registers them to be joined, so sibling
/// components render concurrently; the join runs after the hoist phase,
/// right before the burst. In live mode nothing is awaited during the hoist:
/// invocations and reactive nodes register with the frame's set and resolve
/// through reserved slots, and the frame's barrier is the join.
pub(crate) struct Emitter {
    hoist: TokenStream,
    burst: TokenStream,
    counter: u32,
    mode: EmitMode,
    inline_await: bool,
    join: Vec<Ident>,
}

impl Emitter {
    pub(super) fn new(mode: EmitMode, inline_await: bool) -> Self {
        Self {
            hoist: TokenStream::new(),
            burst: TokenStream::new(),
            counter: 0,
            mode,
            inline_await,
            join: Vec::new(),
        }
    }

    /// Whether this is a live expansion registering with a frame set.
    pub(super) fn live(&self) -> bool {
        self.mode == EmitMode::Live
    }

    /// Whether hoisted futures are awaited where they are bound instead of
    /// being joined after the hoist phase. Blocking mode only.
    pub(super) fn inline_await(&self) -> bool {
        self.inline_await
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

    /// Hoists a binding for `future`, a future of `Result`. Blocking mode
    /// only.
    ///
    /// With inline awaits the binding awaits the future in place. Otherwise
    /// the binding holds the future itself and is rebound to its output by
    /// the join that follows the hoist phase.
    pub(super) fn hoist_future(&mut self, span: Span, ident: &Ident, future: &TokenStream) {
        debug_assert_eq!(self.mode, EmitMode::Blocking);
        if self.inline_await {
            self.hoist(quote_spanned! {span=> let #ident = #future.await?; });
        } else {
            self.hoist(quote_spanned! {span=> let #ident = #future; });
            self.join.push(ident.clone());
        }
    }

    /// Appends pushes on the `__b` builder to the burst phase.
    pub(super) fn burst(&mut self, tokens: TokenStream) {
        self.burst.append_all(tokens);
    }

    /// Returns the statement that awaits the registered futures and rebinds
    /// their identifiers to the outputs, joining when there is more than one.
    fn join_bindings(&self) -> TokenStream {
        match self.join.as_slice() {
            [] => TokenStream::new(),
            [ident] => quote! { let #ident = #ident.await?; },
            idents => quote! {
                let (#(#idents),*) = #topcoat_view::internal::try_join!(#(#idents),*)?;
            },
        }
    }

    /// Returns the view expression: the hoist phase, the join between hoist
    /// and burst, and the burst that builds the instruction block and yields
    /// the view handle.
    ///
    /// In blocking mode the join awaits the futures the hoist registered. In
    /// live mode it is the frame's barrier: every child has handed its view
    /// over, even if work streams on inside.
    pub(super) fn finish(self) -> TokenStream {
        let join = match self.mode {
            EmitMode::Blocking => self.join_bindings(),
            EmitMode::Live => quote! { __refresh.barrier().await?; },
        };
        let Self { hoist, burst, .. } = self;
        quote! {{
            #hoist
            #join
            #topcoat_view::internal::block(__cx, |__b| {
                #burst
            })
        }}
    }

    /// Returns the view expression for a nested position of the same frame:
    /// the hoist and the burst with no join, since the enclosing frame's
    /// barrier covers everything the nested scope registered. Live mode
    /// only.
    pub(super) fn finish_nested(self) -> TokenStream {
        debug_assert_eq!(self.mode, EmitMode::Live);
        let Self { hoist, burst, .. } = self;
        quote! {{
            #hoist
            #topcoat_view::internal::block(__cx, |__b| {
                #burst
            })
        }}
    }
}
