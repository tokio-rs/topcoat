mod eval;
mod expr_assign_deref;
mod expr_closure;
mod expr_deref;
mod expr_field;
mod expr_method_call;
mod expr_param;
mod expr_raw;
mod expr_signal_ref;
mod expr_value;
mod fmt_js;
mod interpreter;

pub use eval::*;
pub use expr_assign_deref::*;
pub use expr_closure::*;
pub use expr_deref::*;
pub use expr_field::*;
pub use expr_method_call::*;
pub use expr_param::*;
pub use expr_raw::*;
pub use expr_signal_ref::*;
pub use expr_value::*;
pub use fmt_js::*;
pub use interpreter::*;

pub trait Expr: Eval + FmtJs {}

impl<T> Expr for T where T: Eval + FmtJs {}

pub trait IntoExpr {
    type Expr;

    fn into_expr(self) -> Self::Expr;
}

impl<T: Expr> IntoExpr for T {
    type Expr = T;

    fn into_expr(self) -> Self::Expr {
        self
    }
}
