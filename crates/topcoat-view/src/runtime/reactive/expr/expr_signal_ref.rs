use crate::runtime::{Expr, Interpreter, IntoExpr, Signal};

pub struct ExprSignalRef<'a, T> {
    signal: &'a Signal<T>,
}

impl<'a, T> Expr for ExprSignalRef<'a, T>
where
    T: Clone,
{
    type Output = T;

    fn evaluate(self, _interpreter: &mut Interpreter) -> Self::Output {
        self.signal.read().clone()
    }
}

impl<'a, T> IntoExpr for &'a Signal<T> {
    type Expr = ExprSignalRef<'a, T>;

    fn into_expr(self) -> Self::Expr {
        ExprSignalRef { signal: self }
    }
}
