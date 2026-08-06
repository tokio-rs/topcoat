use proc_macro2::TokenStream;
use quote::{TokenStreamExt, format_ident, quote};
use syn::Ident;

/// HIR nodes that emit themselves into the two phases of an [`Emitter`].
pub(crate) trait Emit {
    fn emit(&self, emitter: &mut Emitter);
}

/// Collects the two phases a view expression expands to.
///
/// The hoist phase evaluates every expression of the view in source order,
/// including awaiting components and building the views of nested control
/// flow, and binds the results to fresh identifiers. The emit phase then
/// pushes the view's instruction block in one synchronous burst that only
/// reads those bindings. Keeping every `await` out of the emit phase is what
/// guarantees the block lands contiguously in the scope's shared instruction
/// memory.
pub(crate) struct Emitter {
    hoist: TokenStream,
    emit: TokenStream,
    counter: u32,
}

impl Emitter {
    pub(super) fn new() -> Self {
        Self {
            hoist: TokenStream::new(),
            emit: TokenStream::new(),
            counter: 0,
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

    /// Appends instruction pushes to the emit phase.
    pub(super) fn emit(&mut self, tokens: TokenStream) {
        self.emit.append_all(tokens);
    }

    /// Returns the view expression: the hoist phase followed by the burst
    /// that builds the instruction block and yields the view handle.
    pub(super) fn finish(self) -> TokenStream {
        let Self { hoist, emit, .. } = self;
        quote! {{
            #hoist
            __build_view(|__parts| {
                #emit
            })
        }}
    }
}
