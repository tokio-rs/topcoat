use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::Ident;

use crate::view::hir::emit::{Emit, Emitter};

/// A dynamic expression, emitted through its [`ExprKind`]'s builder method.
pub(crate) struct ExprNode {
    pub kind: ExprKind,
    pub tokens: TokenStream,
}

impl Emit for ExprNode {
    fn emit(&self, emitter: &mut Emitter) {
        let ident = emitter.fresh_ident();
        let tokens = &self.tokens;
        let method = self.kind.builder_method();

        emitter.hoist(quote! { let #ident = #tokens; });
        emitter.burst(quote! { __b.#method(#ident); });
    }
}

/// Identifies which builder method an [`ExprNode`] is pushed through when
/// emitted, so the generated code seals the expression with the right
/// position and dispatches the corresponding `*ViewParts` trait.
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
