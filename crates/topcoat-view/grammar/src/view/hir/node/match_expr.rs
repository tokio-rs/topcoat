use syn::{Expr, Pat};

use crate::view::hir::Scope;

/// A `match` whose arm bodies are lowered into nested scopes.
pub(crate) struct MatchExpr {
    pub expr: Box<Expr>,
    pub arms: Vec<MatchArm>,
}

/// A single `pat (if guard)? => body` arm of a [`MatchExpr`].
pub(crate) struct MatchArm {
    pub pat: Pat,
    pub guard: Option<Expr>,
    pub body: Scope,
}
