mod component;
mod expr_node;
mod for_loop;
mod if_else;
mod local;
mod match_expr;
mod statement;
mod static_segment;

pub(crate) use component::*;
pub(crate) use expr_node::*;
pub(crate) use for_loop::*;
pub(crate) use if_else::*;
pub(crate) use local::*;
pub(crate) use match_expr::*;
use quote::ToTokens;
pub(crate) use statement::*;
pub(crate) use static_segment::*;
use syn::Ident;

use super::{
    bindings::mentions,
    emit::{Emit, Emitter},
};

/// A single node of a lowered [`View`](crate::view::View). Produced by
/// [`ViewBuilder`](super::ViewBuilder), emitted by [`Scope`](super::Scope).
pub(crate) enum Node {
    /// Literal markup, emitted verbatim.
    StaticSegment(StaticSegment),
    /// A component invocation, emitted through the props builder.
    Component(Component),
    /// A dynamic expression, emitted through its [`ExprKind`]'s position.
    ExprNode(ExprNode),
    /// A `let pat = expr;` binding, in scope for the nodes that follow it.
    Local(Local),
    /// A verbatim Rust statement.
    Statement(Statement),
    /// A `for` loop whose body is lowered into a nested scope.
    ForLoop(ForLoop),
    /// An `if`/`else` whose branches are lowered into nested scopes.
    IfElse(IfElse),
    /// A `match` whose arm bodies are lowered into nested scopes.
    MatchExpr(MatchExpr),
}

impl Node {
    /// Whether any expression of this node, or of a scope nested in it,
    /// mentions `ident`.
    ///
    /// A pattern is not an expression: it may bind the name again, but it
    /// never reads it.
    pub(super) fn mentions(&self, ident: &Ident) -> bool {
        match self {
            Self::StaticSegment(_) => false,
            Self::Component(node) => {
                node.named_args
                    .iter()
                    .any(|arg| mentions(&arg.value.to_token_stream(), ident))
                    || node
                        .key
                        .as_ref()
                        .is_some_and(|key| mentions(&key.value.to_token_stream(), ident))
                    || node
                        .children
                        .as_ref()
                        .is_some_and(|children| children.mentions(ident))
            }
            Self::ExprNode(node) => mentions(&node.tokens, ident),
            Self::Local(node) => mentions(&node.expr.to_token_stream(), ident),
            Self::Statement(node) => mentions(&node.tokens, ident),
            Self::ForLoop(node) => {
                mentions(&node.expr.to_token_stream(), ident) || node.body.mentions(ident)
            }
            Self::IfElse(node) => {
                mentions(&node.expr.to_token_stream(), ident)
                    || node.then_branch.mentions(ident)
                    || node.else_branch.mentions(ident)
            }
            Self::MatchExpr(node) => {
                mentions(&node.expr.to_token_stream(), ident)
                    || node.arms.iter().any(|arm| {
                        arm.guard
                            .as_ref()
                            .is_some_and(|guard| mentions(&guard.to_token_stream(), ident))
                            || arm.body.mentions(ident)
                    })
            }
        }
    }
}

impl Emit for Node {
    fn emit(&self, emitter: &mut Emitter<'_>) {
        match self {
            Self::StaticSegment(node) => node.emit(emitter),
            Self::Component(node) => node.emit(emitter),
            Self::ExprNode(node) => node.emit(emitter),
            Self::Local(node) => node.emit(emitter),
            Self::Statement(node) => node.emit(emitter),
            Self::ForLoop(node) => node.emit(emitter),
            Self::IfElse(node) => node.emit(emitter),
            Self::MatchExpr(node) => node.emit(emitter),
        }
    }
}
