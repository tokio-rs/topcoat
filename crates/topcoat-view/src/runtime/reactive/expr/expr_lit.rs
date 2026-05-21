use crate::runtime::{Expr, Interpreter, IntoExpr};

pub struct ExprLit<T>(T);

impl<T> Expr for ExprLit<T> {
    type Output = T;

    fn evaluate(self, _interpreter: &mut Interpreter) -> Self::Output {
        self.0
    }
}

macro_rules! impl_primitive {
    ($ty:ty) => {
        impl IntoExpr for $ty {
            type Expr = ExprLit<Self>;

            fn into_expr(self) -> Self::Expr {
                ExprLit(self)
            }
        }
    };
}

impl_primitive!(bool);
impl_primitive!(f64);
impl_primitive!(&'static str);
