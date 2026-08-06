use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::Ident;

use crate::view::hir::emit::{Emit, Emitter};

/// A dynamic expression, emitted through its [`ExprKind`]'s helper.
pub(crate) struct ExprNode {
    pub kind: ExprKind,
    pub tokens: TokenStream,
}

impl Emit for ExprNode {
    fn emit(&self, emitter: &mut Emitter) {
        let ident = emitter.fresh_ident();
        let tokens = &self.tokens;
        let helper = self.kind.helper();

        emitter.hoist(quote! { let #ident = #tokens; });
        emitter.emit(quote! { #helper(__cx, __parts, #ident); });
    }
}

/// Identifies which `internal` helper an [`ExprNode`] should be wrapped in
/// when emitted, so the generated code uses the matching `__*` function and
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
    pub(crate) fn helper(self) -> Ident {
        let name = match self {
            Self::Node => "__node",
            Self::ElementName => "__element_name",
            Self::Attribute => "__attribute",
            Self::AttributeUnescaped => "__attribute_unescaped",
            Self::AttributeKey => "__attribute_key",
            Self::AttributeValue => "__attribute_value",
            Self::Attributes => "__attributes",
        };
        Ident::new(name, Span::call_site())
    }
}
