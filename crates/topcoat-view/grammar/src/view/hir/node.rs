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

/// A single node of a lowered [`View`](crate::view::View). Produced by
/// [`ViewBuilder`](super::ViewBuilder), emitted by [`Scope`].
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
    pub(super) fn contains_component(&self) -> bool {
        match self {
            Self::Component(_) => true,
            Self::ForLoop(ForLoop { body, .. }) => body.contains_component(),
            Self::IfElse(IfElse {
                then_branch,
                else_branch,
                ..
            }) => then_branch.contains_component() || else_branch.contains_component(),
            Self::MatchExpr(MatchExpr { arms, .. }) => {
                arms.iter().any(|arm| arm.body.contains_component())
            }
            Self::StaticSegment(_) | Self::ExprNode(_) | Self::Local(_) | Self::Statement(_) => {
                false
            }
        }
    }
}
