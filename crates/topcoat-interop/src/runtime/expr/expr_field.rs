use std::marker::PhantomData;

use crate::runtime::{Eval, Expr, FmtJs, Formatter, Interpreter};

/// A `receiver.field` access on a handler-internal value. The accessor closure
/// passed to `new` exists purely so rustc resolves `T` from the receiver's
/// real type — it is never invoked. Server-side `eval` is unreachable.
pub struct ExprField<R, T> {
    receiver: R,
    name: &'static str,
    _phantom: PhantomData<fn() -> T>,
}

impl<R, T> ExprField<R, T>
where
    R: Expr,
{
    pub fn new<F>(receiver: R, name: &'static str, _accessor: F) -> Self
    where
        F: FnOnce(R::Output) -> T,
    {
        Self {
            receiver,
            name,
            _phantom: PhantomData,
        }
    }
}

impl<R, T> Eval for ExprField<R, T>
where
    R: Eval,
{
    type Output = T;

    fn eval(self, _interpreter: &mut Interpreter) -> Self::Output {
        unreachable!("ExprField::eval called server-side; handler bodies do not run during SSR")
    }
}

impl<R, T> FmtJs for ExprField<R, T>
where
    R: FmtJs,
{
    fn fmt_js(&self, f: &mut Formatter<'_>) {
        f.write_char('(');
        self.receiver.fmt_js(f);
        f.write_str(").");
        f.write_str(self.name);
    }
}
