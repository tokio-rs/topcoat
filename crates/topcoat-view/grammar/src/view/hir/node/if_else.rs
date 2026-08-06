use syn::Expr;

use crate::view::hir::Scope;

/// An `if`/`else` whose branches are lowered into nested scopes.
pub(crate) struct IfElse {
    pub expr: Expr,
    pub then_branch: Scope,
    pub else_branch: Scope,
}
