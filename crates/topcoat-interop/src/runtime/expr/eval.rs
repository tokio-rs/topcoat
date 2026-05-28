use crate::runtime::Interpreter;

/// An expression that can be evaluated in Rust, i.e. on the server side.
pub trait Eval {
    type Output;

    fn eval(self, interpreter: &mut Interpreter) -> Self::Output;
}
