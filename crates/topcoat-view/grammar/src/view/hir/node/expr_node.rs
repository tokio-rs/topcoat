use proc_macro2::{Span, TokenStream};
use syn::Ident;

/// A dynamic expression, emitted through its [`ExprKind`]'s helper.
pub(crate) struct ExprNode {
    pub kind: ExprKind,
    pub tokens: TokenStream,
}

/// Identifies which `internal` helper an [`ExprNode`] should be wrapped in
/// when emitted, so the generated code uses the matching `__*` function and
/// the corresponding `*ViewParts` trait.
#[derive(Copy, Clone)]
pub(crate) enum ExprKind {
    Unescaped,
    Node,
    View,
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
            Self::Unescaped => "__unescaped",
            Self::Node => "__node",
            Self::View => "__view",
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
