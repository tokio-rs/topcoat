use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::Ident;
use topcoat_core_grammar::paths::topcoat_view;

use crate::view::hir::emit::{Emit, Emitter};

/// A dynamic expression: a node position is split into the parts the burst
/// pushes and the unit the join drives, any other position is pushed
/// through its [`ExprKind`]'s builder method.
pub(crate) struct ExprNode {
    pub kind: ExprKind,
    pub tokens: TokenStream,
}

impl ExprNode {
    /// Whether this expression fills a node position.
    pub(crate) fn is_node_position(&self) -> bool {
        matches!(self.kind, ExprKind::Node)
    }
}

impl Emit for ExprNode {
    fn emit(&self, emitter: &mut Emitter) {
        let ident = emitter.fresh_ident();
        let tokens = &self.tokens;

        match self.kind {
            ExprKind::Node => {
                // The value's type decides what it is: a parts value is
                // pushed at the position and the join drives the unit `()`,
                // a view is driven by the join and its content is spliced
                // at the position. The burst does both, and one of them is
                // a no-op.
                let parts = format_ident!("{ident}_parts");
                emitter.hoist(quote! {
                    let (#parts, #ident) = #topcoat_view::internal::NodeClassify::classify(#tokens);
                });
                emitter.burst(quote! { __b.node(#parts); });
                emitter.unit(Span::call_site(), &ident);
            }
            kind => {
                let method = kind.builder_method();
                emitter.hoist(quote! { let #ident = #tokens; });
                emitter.burst(quote! { __b.#method(#ident); });
            }
        }
    }
}

/// Identifies how an [`ExprNode`] is emitted: a node position is classified
/// and joins the template's units, every other position maps to the builder
/// method that seals the expression with the right position and dispatches
/// the corresponding `*ViewParts` trait.
#[derive(Copy, Clone)]
pub(crate) enum ExprKind {
    Node,
    ElementName,
    Attribute,
    AttributeUnescaped,
    AttributeKey,
    AttributeValue,
    Attributes,
}

impl ExprKind {
    /// Returns the builder method a position other than a node position is
    /// pushed through.
    ///
    /// # Panics
    ///
    /// Panics for a node position, which is joined as a unit instead.
    pub(crate) fn builder_method(self) -> Ident {
        let name = match self {
            Self::Node => panic!("a node position is joined as a unit, not pushed"),
            Self::ElementName => "element_name",
            Self::Attribute => "attribute",
            Self::AttributeUnescaped => "attribute_unescaped",
            Self::AttributeKey => "attribute_key",
            Self::AttributeValue => "attribute_value",
            Self::Attributes => "attributes",
        };
        Ident::new(name, Span::call_site())
    }
}
