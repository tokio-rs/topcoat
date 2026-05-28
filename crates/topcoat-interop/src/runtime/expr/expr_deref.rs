use crate::runtime::{Eval, FmtJs, Formatter, Interpreter};

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

impl<E> Eval for ExprDeref<E>
where
    E: Eval,
    E::Output: ExprDerefTarget,
{
    type Output = <E::Output as ExprDerefTarget>::Target;

    fn eval(self, interpreter: &mut Interpreter) -> Self::Output {
        self.0.eval(interpreter).expr_deref()
    }
}

impl<E> FmtJs for ExprDeref<E>
where
    E: FmtJs,
{
    fn fmt_js(&self, f: &mut Formatter<'_>) {
        // In JS, maverick signal handles are callable; reading is `handle()`.
        f.write_char('(');
        self.0.fmt_js(f);
        f.write_str(")()");
    }
}
