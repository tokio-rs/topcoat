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
pub(crate) use statement::*;
pub(crate) use static_segment::*;

use super::emit::{Emit, Emitter};

/// A single node of a lowered [`View`](crate::view::View). Produced by
/// [`ViewBuilder`](super::ViewBuilder), emitted by [`Scope`](super::Scope).
pub(crate) enum Node {
    /// Literal markup, emitted verbatim.
    StaticSegment(StaticSegment),
    /// A component invocation, emitted through the props builder.
    Component(Component),
    /// A dynamic expression, emitted through its [`ExprKind`]'s helper.
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
    /// Whether this node renders components, in the node itself or anywhere
    /// under its nested scopes, so emitting it produces a future to await.
    pub(crate) fn is_async(&self) -> bool {
        match self {
            Self::Component(_) => true,
            Self::ForLoop(node) => node.body.is_async(),
            Self::IfElse(node) => node.then_branch.is_async() || node.else_branch.is_async(),
            Self::MatchExpr(node) => node.arms.iter().any(|arm| arm.body.is_async()),
            Self::StaticSegment(_) | Self::ExprNode(_) | Self::Local(_) | Self::Statement(_) => {
                false
            }
        }
    }
}

impl Emit for Node {
    fn emit(&self, emitter: &mut Emitter) {
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
