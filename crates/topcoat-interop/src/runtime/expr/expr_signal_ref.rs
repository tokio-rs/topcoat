use std::fmt::Write;

use crate::runtime::{Expr, ExprDerefAssignTarget, ExprDerefTarget, Interpreter, IntoExpr, Value};

// pub struct ExprSignalRef<'a, T> {
//     signal: &'a Signal<T>,
// }
//
// impl<'a, T> Expr for ExprSignalRef<'a, T> {
//     type Output = &'a Signal<T>;
//
//     fn eval(self, _interpreter: &mut Interpreter) -> Self::Output {
//         self.signal
//     }
//
//     fn to_js(&self, out: &mut String) {
//         let id =
//             serde_json::to_string(&self.signal.id()).expect("signal id is serializable as JSON");
//         write!(out, "__context.signal({id})").unwrap();
//     }
// }
//
// impl<'a, T> IntoExpr for &'a Signal<T> {
//     type Expr = ExprSignalRef<'a, T>;
//
//     fn into_expr(self) -> Self::Expr {
//         ExprSignalRef { signal: self }
//     }
// }
//
// impl<'a, T> ExprDerefTarget for &'a Signal<T>
// where
//     T: Value,
// {
//     type Target = &'a T::Surrogate;
//
//     fn expr_deref(self) -> Self::Target {
//         self.read()
//     }
// }
//
// impl<T> ExprDerefAssignTarget for &Signal<T>
// where
//     T: Value,
//     T::Surrogate: Sized,
// {
//     type Value = T::Surrogate;
//
//     fn expr_deref_assign(self, _value: Self::Value) {
//         unreachable!(
//             "ExprDerefAssignTarget::expr_deref_assign called server-side; handler bodies do not run during SSR"
//         )
//     }
// }
