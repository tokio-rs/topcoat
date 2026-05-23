use std::marker::PhantomData;

use crate::{Expr, Interpreter, JsCallable};

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
    R::Output: JsCallable,
    F: FnOnce(R::Output) -> T,
{
    type Output = T;

    fn eval(self, interpreter: &mut Interpreter) -> Self::Output {
        let receiver = self.receiver.eval(interpreter);
        (self.accessor)(receiver)
    }

    fn to_js(&self, out: &mut String) {
        out.push('(');
        self.receiver.to_js(out);
        out.push(')');
        R::Output::js_call(self.method, out);
    }
}
