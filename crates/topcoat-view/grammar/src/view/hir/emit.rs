use proc_macro2::{Span, TokenStream};
use quote::{TokenStreamExt, format_ident, quote, quote_spanned};
use syn::Ident;

/// HIR nodes that emit themselves into the two phases of an [`Emitter`].
pub(crate) trait Emit {
    fn emit(&self, emitter: &mut Emitter);
}

/// Collects the two phases a view expression expands to.
///
/// The hoist phase evaluates every expression of the view in source order,
/// including building the views of nested control flow, and binds the results
/// to fresh identifiers. The emit phase then pushes the view's instruction
/// block in one synchronous burst that only reads those bindings. Keeping
/// every `await` out of the emit phase is what guarantees the block lands
/// contiguously in the scope's shared instruction memory.
///
/// Component renders are futures. With inline awaits, the hoist phase awaits
/// each one where it is bound. Without them, the hoist phase only binds the
/// futures and registers them to be joined, so sibling components render
/// concurrently; the join runs after the hoist phase, right before the emit
/// phase's burst.
pub(crate) struct Emitter {
    hoist: TokenStream,
    emit: TokenStream,
    counter: u32,
    inline_await: bool,
    join: Vec<Ident>,
}

impl Emitter {
    pub(super) fn new(inline_await: bool) -> Self {
        Self {
            hoist: TokenStream::new(),
            emit: TokenStream::new(),
            counter: 0,
            inline_await,
            join: Vec::new(),
        }
    }

    /// Whether hoisted futures are awaited where they are bound instead of
    /// being joined after the hoist phase.
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

    /// Hoists a binding for `future`, a future of `Result`.
    ///
    /// With inline awaits the binding awaits the future in place. Otherwise
    /// the binding holds the future itself and is rebound to its output by
    /// the join that follows the hoist phase.
    pub(super) fn hoist_future(&mut self, span: Span, ident: &Ident, future: &TokenStream) {
        if self.inline_await {
            self.hoist(quote_spanned! {span=> let #ident = #future.await?; });
        } else {
            self.hoist(quote_spanned! {span=> let #ident = #future; });
            self.join.push(ident.clone());
        }
    }

    /// Appends instruction pushes to the emit phase.
    pub(super) fn emit(&mut self, tokens: TokenStream) {
        self.emit.append_all(tokens);
    }

    /// Returns the statement that awaits the registered futures and rebinds
    /// their identifiers to the outputs, joining when there is more than one.
    fn join_bindings(&self) -> TokenStream {
        match self.join.as_slice() {
            [] => TokenStream::new(),
            [ident] => quote! { let #ident = #ident.await?; },
            idents => quote! { let (#(#idents),*) = __try_join!(#(#idents),*)?; },
        }
    }

    /// Returns the view expression: the hoist phase, the join of any
    /// registered futures, and the burst that builds the instruction block
    /// and yields the view handle.
    pub(super) fn finish(self) -> TokenStream {
        let join = self.join_bindings();
        let Self { hoist, emit, .. } = self;
        quote! {{
            #hoist
            #join
            __build_view(|__parts| {
                #emit
            })
        }}
    }
}
