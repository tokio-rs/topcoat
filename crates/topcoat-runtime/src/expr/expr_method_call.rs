use std::marker::PhantomData;

use serde::{Serialize, Serializer, ser::SerializeStruct};

use crate::{Expr, Interpreter};

/// A `receiver.method()` call. Only zero-argument methods are supported. The
/// accessor closure passed to `new` carries the real implementation so the
/// server-side `eval` can run it and rustc can resolve `T` from the receiver
/// type.
pub struct ExprMethodCall<R, F, T> {
    receiver: R,
    method: &'static str,
    accessor: F,
    _phantom: PhantomData<fn() -> T>,
}

impl<R, F, T> ExprMethodCall<R, F, T>
where
    R: Expr,
{
    pub fn new(receiver: R, method: &'static str, accessor: F) -> Self
    where
        F: FnOnce(R::Output) -> T,
    {
        Self {
            receiver,
            method,
            accessor,
            _phantom: PhantomData,
        }
    }
}

impl<R, F, T> Expr for ExprMethodCall<R, F, T>
where
    R: Expr,
    F: FnOnce(R::Output) -> T,
{
    type Output = T;

    fn eval(self, interpreter: &mut Interpreter) -> Self::Output {
        let receiver = self.receiver.eval(interpreter);
        (self.accessor)(receiver)
    }
}

impl<R, F, T> Serialize for ExprMethodCall<R, F, T>
where
    R: Serialize,
{
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("ExprMethodCall", 3)?;
        s.serialize_field("type", "MethodCall")?;
        s.serialize_field("receiver", &self.receiver)?;
        s.serialize_field("method", self.method)?;
        s.end()
    }
}
