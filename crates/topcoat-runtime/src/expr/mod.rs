mod expr_assign_deref;
mod expr_closure;
mod expr_deref;
mod expr_field;
mod expr_lit;
mod expr_method_call;
mod expr_param;
mod expr_signal_ref;
mod interpreter;

pub use expr_assign_deref::*;
pub use expr_closure::*;
pub use expr_deref::*;
pub use expr_field::*;
pub use expr_lit::*;
pub use expr_method_call::*;
pub use expr_param::*;
pub use expr_signal_ref::*;
pub use interpreter::*;

use serde::Serialize;

pub trait Expr: Serialize {
    type Output;

    fn eval(self, interp: &mut Interpreter) -> Self::Output;
}

pub trait IntoExpr {
    type Expr;

    fn into_expr(self) -> Self::Expr;
}
