use proc_macro2::{Span, TokenStream};
use syn::{Expr, Ident, Pat};

use super::Scope;

/// A single node of a lowered [`View`](crate::view::View). Produced by
/// [`ViewBuilder`](super::ViewBuilder), emitted by [`Scope`].
pub(crate) enum Node {
    /// Literal markup, emitted verbatim.
    Static { string: String },
    /// A dynamic expression, emitted through its [`ExprKind`]'s helper.
    Expr { kind: ExprKind, tokens: TokenStream },
    /// A `let pat = expr;` binding, in scope for the nodes that follow it.
    Local { pat: Pat, expr: Box<Expr> },
    /// A verbatim Rust statement.
    Statement { tokens: TokenStream },
    /// A `for` loop whose body is lowered into a nested scope.
    For {
        pat: Pat,
        expr: Box<Expr>,
        body: Scope,
    },
    /// An `if`/`else` whose branches are lowered into nested scopes.
    If {
        expr: Expr,
        then_branch: Scope,
        else_branch: Scope,
    },
    /// A `match` whose arm bodies are lowered into nested scopes.
    Match {
        expr: Box<Expr>,
        arms: Vec<MatchArm>,
    },
}

/// A single `pat (if guard)? => body` arm of a [`Node::Match`].
pub(crate) struct MatchArm {
    pub pat: Pat,
    pub guard: Option<Expr>,
    pub body: Scope,
}

/// Identifies which `internal` helper a [`Node::Expr`] should be wrapped in
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
    pub(super) fn helper(self) -> Ident {
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
