use serde::{Serialize, Serializer, ser::SerializeStruct};

use crate::{Expr, Interpreter, IntoExpr, Signal};

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

impl<T> Serialize for ExprSignalRef<'_, T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("ExprSignalRef", 2)?;
        s.serialize_field("type", "SignalRef")?;
        s.serialize_field("id", &self.signal.id())?;
        s.end()
    }
}

impl<'a, T> IntoExpr for &'a Signal<T> {
    type Expr = ExprSignalRef<'a, T>;

    fn into_expr(self) -> Self::Expr {
        ExprSignalRef { signal: self }
    }
}
