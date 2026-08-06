use syn::{Expr, Pat};

use crate::view::hir::Scope;

/// A `for` loop whose body is lowered into a nested scope.
pub(crate) struct ForLoop {
    pub pat: Pat,
    pub expr: Box<Expr>,
    pub body: Scope,
}
