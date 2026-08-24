use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::Ident;

use crate::view::hir::emit::{Emit, Emitter};

/// A dynamic expression: a node position is driven as a joined unit, any
/// other position is pushed through its [`ExprKind`]'s builder method.
pub(crate) struct ExprNode {
    pub kind: ExprKind,
    pub tokens: TokenStream,
}

impl Emit for ExprNode {
    fn emit(&self, emitter: &mut Emitter) {
        let ident = emitter.fresh_ident();
        let tokens = &self.tokens;

        emitter.hoist(quote! { let #ident = #tokens; });
        match self.kind {
            ExprKind::Node => emitter.unit(Span::call_site(), &ident),
            kind => {
                let method = kind.builder_method();
                emitter.burst(quote! { __b.#method(#ident); });
            }
        }
    }
}

/// Identifies how an [`ExprNode`] is emitted: a node position joins the
/// template's units, every other position maps to the builder method that
/// seals the expression with the right position and dispatches the
/// corresponding `*ViewParts` trait.
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
    pub(crate) fn builder_method(self) -> Ident {
        let name = match self {
            Self::Node => "node",
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
