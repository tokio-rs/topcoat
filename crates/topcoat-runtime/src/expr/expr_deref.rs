use serde::{Serialize, Serializer, ser::SerializeStruct};

use crate::{Expr, Interpreter};

pub trait ExprDerefTarget {
    type Target;

    fn expr_deref(self) -> Self::Target;
}

pub struct ExprDeref<E>(E);

impl<E> ExprDeref<E> {
    pub fn new(inner: E) -> Self {
        Self(inner)
    }
}

impl<E> Expr for ExprDeref<E>
where
    E: Expr,
    E::Output: ExprDerefTarget,
{
    type Output = <E::Output as ExprDerefTarget>::Target;

    fn eval(self, interpreter: &mut Interpreter) -> Self::Output {
        self.0.eval(interpreter).expr_deref()
    }
}

impl<E: Serialize> Serialize for ExprDeref<E> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("ExprDeref", 2)?;
        s.serialize_field("type", "Deref")?;
        s.serialize_field("inner", &self.0)?;
        s.end()
    }
}
