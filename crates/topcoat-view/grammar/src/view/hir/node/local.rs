use syn::{Expr, Pat};

/// A `let pat = expr;` binding, in scope for the nodes that follow it.
pub(crate) struct Local {
    pub pat: Pat,
    pub expr: Box<Expr>,
}
