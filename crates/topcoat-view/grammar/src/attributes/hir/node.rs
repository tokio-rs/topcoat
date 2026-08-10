use proc_macro2::TokenStream;
use syn::{Expr, Pat};

use super::Scope;

/// A single node of a lowered [`Attributes`](crate::attributes::Attributes)
/// list. Produced by [`AttributeBuilder`](super::AttributeBuilder), emitted by
/// [`Scope`].
pub(crate) enum Node {
    /// A self-contained block that inserts `capacity` entries into `__attrs`.
    Insert {
        tokens: TokenStream,
        capacity: usize,
    },
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
        cond: Expr,
        then_branch: Scope,
        else_branch: Scope,
    },
    /// A `match` whose arm bodies are lowered into nested scopes.
    Match {
        expr: Box<Expr>,
        arms: Vec<MatchArm>,
    },
}

impl Node {
    /// The number of entries this node is guaranteed to insert.
    pub(super) fn capacity(&self) -> usize {
        match self {
            Self::Insert { capacity, .. } => *capacity,
            Self::Local { .. } | Self::Statement { .. } | Self::For { .. } => 0,
            Self::If {
                then_branch,
                else_branch,
                ..
            } => then_branch.capacity().min(else_branch.capacity()),
            Self::Match { arms, .. } => arms
                .iter()
                .map(|arm| arm.body.capacity())
                .min()
                .unwrap_or_default(),
        }
    }
}

/// A single `pat (if guard)? => body` arm of a [`Node::Match`].
pub(crate) struct MatchArm {
    pub pat: Pat,
    pub guard: Option<Expr>,
    pub body: Scope,
}
