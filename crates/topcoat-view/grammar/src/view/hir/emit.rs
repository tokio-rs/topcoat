use proc_macro2::TokenStream;
use quote::{TokenStreamExt, format_ident, quote};
use syn::Ident;

use super::Scope;

/// HIR nodes that emit themselves into an [`Emitter`].
pub(crate) trait Emit {
    fn emit(&self, emitter: &mut Emitter<'_>);
}

/// How often a scope renders per pass over the template it belongs to,
/// which decides how a node position in it collects what it leaves to
/// drive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Placement {
    /// Exactly once: the template's own nodes.
    Once,
    /// At most once: the body of an `if` branch or `match` arm.
    Conditional,
    /// Any number of times: the body of a `for` loop, at any depth.
    Repeated,
}

impl Placement {
    /// The placement of a branch nested in a scope with this placement.
    pub(crate) fn branch(self) -> Self {
        match self {
            Self::Once | Self::Conditional => Self::Conditional,
            Self::Repeated => Self::Repeated,
        }
    }
}

/// Collects the straight-line code a scope's nodes expand to.
///
/// The code runs where the scope renders, pushing the block's parts
/// through the `__b` builder in source order. A node position that leaves a
/// view to drive stores it into a collector through [`site`](Self::site);
/// the collectors of one template are shared by every scope nested in it,
/// so a nested scope emits through [`nested`](Self::nested).
pub(crate) struct Emitter<'a> {
    body: TokenStream,
    sites: &'a mut Sites,
    placement: Placement,
}

impl<'a> Emitter<'a> {
    pub(super) fn new(sites: &'a mut Sites, placement: Placement) -> Self {
        Self {
            body: TokenStream::new(),
            sites,
            placement,
        }
    }

    /// Appends statements to the scope's code.
    pub(super) fn push(&mut self, tokens: TokenStream) {
        self.body.append_all(tokens);
    }

    /// Registers a node position and returns the statement storing
    /// `pending`, what the position leaves to drive, into its collector.
    ///
    /// A position rendered exactly once keeps its pending in a plain
    /// binding; one in a branch stores into an `Option` that stays `None`
    /// when the branch is not taken; one in a loop body pushes into a `Vec`
    /// with an entry per pass.
    pub(super) fn site(&mut self, pending: &TokenStream) -> TokenStream {
        let ident = self.sites.next(self.placement);
        match self.placement {
            Placement::Once => quote! { let #ident = #pending; },
            Placement::Conditional => {
                quote! { #ident = ::core::option::Option::Some(#pending); }
            }
            Placement::Repeated => quote! { #ident.push(#pending); },
        }
    }

    /// Emits the nodes of a scope nested in this one, sharing this
    /// template's collectors, and returns their code.
    pub(super) fn nested(&mut self, scope: &Scope) -> TokenStream {
        scope.emit_nodes(self.sites)
    }

    /// Emits a control-flow node whose nested scopes `f` emits, preceded by
    /// the declarations of the collectors those scopes store into.
    ///
    /// A collector must exist before the node renders, and must be declared
    /// after every binding the views it collects may borrow, so it is
    /// declared right before the outermost control-flow node its position
    /// sits under: the one emitted from the template's own scope. A node
    /// nested in another leaves the declarations to the outermost.
    pub(super) fn control_flow(&mut self, f: impl FnOnce(&mut Self) -> TokenStream) {
        let mark = self.sites.collectors.len();
        let node = f(self);
        if self.placement == Placement::Once {
            let declarations = self.sites.declarations_since(mark);
            self.push(declarations);
        }
        self.push(node);
    }

    /// Returns `expr` wrapped to evaluate with the block suspended, for an
    /// expression that awaits.
    ///
    /// Nothing else can build into the buffer while the block holds it, so
    /// the builder hands it back for the duration and takes it back after.
    /// The wrapping is a call, so the temporaries of `expr` live as long as
    /// the statement around it, as they would unwrapped.
    pub(super) fn awaited(expr: &TokenStream) -> TokenStream {
        quote! { __b.suspended().resumed(#expr) }
    }

    /// Returns the scope's code.
    pub(super) fn finish(self) -> TokenStream {
        self.body
    }
}

/// The node positions of one template that leave a view to drive: one
/// collector per position.
pub(crate) struct Sites {
    collectors: Vec<Collector>,
}

/// One collector: the binding a node position stores its pending into.
struct Collector {
    ident: Ident,
    placement: Placement,
}

impl Sites {
    pub(super) fn new() -> Self {
        Self {
            collectors: Vec::new(),
        }
    }

    /// Registers a position with the given placement and returns its
    /// collector's identifier.
    fn next(&mut self, placement: Placement) -> Ident {
        let ident = format_ident!("__s{}", self.collectors.len());
        self.collectors.push(Collector {
            ident: ident.clone(),
            placement,
        });
        ident
    }

    /// Returns the declarations of the collectors registered since `mark`
    /// that must exist before their position renders.
    ///
    /// A position rendered exactly once declares its binding where it
    /// renders, so it needs none.
    fn declarations_since(&self, mark: usize) -> TokenStream {
        self.collectors[mark..]
            .iter()
            .map(|collector| {
                let ident = &collector.ident;
                match collector.placement {
                    Placement::Once => TokenStream::new(),
                    Placement::Conditional => {
                        quote! { let mut #ident = ::core::option::Option::None; }
                    }
                    Placement::Repeated => quote! { let mut #ident = ::std::vec::Vec::new(); },
                }
            })
            .collect()
    }

    /// Returns the tuple of every collector, in position order, as the
    /// template's pending.
    ///
    /// `Pending` is implemented for tuples up to a fixed arity, so a longer
    /// list nests its tail into the last element.
    pub(super) fn tuple(&self) -> TokenStream {
        fn tuple(idents: &[&Ident]) -> TokenStream {
            const ARITY: usize = 16;
            if idents.len() <= ARITY {
                return quote! { (#(#idents,)*) };
            }
            let (head, tail) = idents.split_at(ARITY - 1);
            let tail = tuple(tail);
            quote! { (#(#head,)* #tail,) }
        }

        let idents: Vec<_> = self
            .collectors
            .iter()
            .map(|collector| &collector.ident)
            .collect();
        tuple(&idents)
    }
}
