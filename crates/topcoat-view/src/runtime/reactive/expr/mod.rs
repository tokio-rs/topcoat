mod expr_lit;
mod expr_signal_ref;
mod interpreter;

pub use expr_lit::*;
pub use expr_signal_ref::*;
pub use interpreter::*;

pub trait Expr {
    type Output;

    fn evaluate(self, interpreter: &mut Interpreter) -> Self::Output;
}

pub trait IntoExpr {
    type Expr;

    fn into_expr(self) -> Self::Expr;
}
