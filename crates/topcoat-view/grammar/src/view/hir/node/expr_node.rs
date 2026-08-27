use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::Ident;
use topcoat_core_grammar::paths::topcoat_view;

use crate::view::hir::{
    bindings::awaits,
    emit::{Emit, Emitter},
};

/// A dynamic expression: a node position renders through `NodePosition`,
/// which decides by its type whether it is pushed at once or driven later;
/// any other position is pushed through its [`ExprKind`]'s builder method.
pub(crate) struct ExprNode {
    pub kind: ExprKind,
    pub tokens: TokenStream,
}

impl Emit for ExprNode {
    fn emit(&self, emitter: &mut Emitter<'_>) {
        let tokens = &self.tokens;
        let awaits = awaits(tokens);
        match self.kind {
            ExprKind::Node => {
                let value = if awaits {
                    Emitter::awaited(tokens)
                } else {
                    tokens.clone()
                };
                let store = emitter.site(&quote! {
                    #topcoat_view::internal::NodePosition::render(#value, &mut __b)
                });
                emitter.push(store);
            }
            kind => {
                let method = kind.builder_method();
                if awaits {
                    // The builder cannot be the receiver of the push while
                    // the suspension borrows it, so the value is bound
                    // first.
                    let value = Emitter::awaited(tokens);
                    emitter.push(quote! {
                        let __awaited = #value;
                        __b.#method(__awaited);
                    });
                } else {
                    emitter.push(quote! { __b.#method(#tokens); });
                }
            }
        }
    }
}

/// Identifies how an [`ExprNode`] is emitted: a node position renders
/// through `NodePosition`, every other position maps to the builder method
/// that seals the expression with the right position and dispatches the
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
    /// Returns the builder method a position other than a node position is
    /// pushed through.
    ///
    /// # Panics
    ///
    /// Panics for a node position, which renders through `NodePosition`
    /// instead.
    pub(crate) fn builder_method(self) -> Ident {
        let name = match self {
            Self::Node => panic!("a node position renders through `NodePosition`, not a method"),
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
