use serde::{Serialize, Serializer, ser::SerializeStruct};

use crate::{Expr, Interpreter, IntoExpr};

pub struct ExprLit<T>(T);

impl<T> Expr for ExprLit<T> {
    type Output = T;

    fn evaluate(self, _interpreter: &mut Interpreter) -> Self::Output {
        self.0
    }
}

impl<T: Serialize> Serialize for ExprLit<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("ExprLit", 2)?;
        s.serialize_field("type", "Lit")?;
        s.serialize_field("value", &self.0)?;
        s.end()
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
