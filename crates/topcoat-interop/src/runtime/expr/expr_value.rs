use crate::runtime::{Eval, Interpreter};

pub struct ExprValue<T>(T);

impl<T> ExprValue<T> {
    #[inline]
    pub const fn new(inner: T) -> Self {
        Self(inner)
    }
}

impl<T> Eval for ExprValue<T> {
    type Output = T;

    #[inline]
    fn eval(self, _interpreter: &mut Interpreter) -> Self::Output {
        self.0
    }
}
