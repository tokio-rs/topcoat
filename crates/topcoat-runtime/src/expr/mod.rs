mod expr_assign_deref;
mod expr_closure;
mod expr_deref;
mod expr_field;
mod expr_lit;
mod expr_method_call;
mod expr_param;
mod expr_signal_ref;
mod interpreter;

pub use expr_assign_deref::*;
pub use expr_closure::*;
pub use expr_deref::*;
pub use expr_field::*;
pub use expr_lit::*;
pub use expr_method_call::*;
pub use expr_param::*;
pub use expr_signal_ref::*;
pub use interpreter::*;

/// A reactive expression. `eval` runs server-side for SSR-time evaluation
/// (only meaningful for read-position nodes); `to_js` emits a JavaScript
/// fragment that the browser compiles via `new Function('__context', …)`.
pub trait Expr {
    type Output;

    fn eval(self, interp: &mut Interpreter) -> Self::Output;
    fn to_js(&self, out: &mut String);
}

pub trait IntoExpr {
    type Expr;

    fn into_expr(self) -> Self::Expr;
}

impl<T: Expr> IntoExpr for T {
    type Expr = T;

    fn into_expr(self) -> Self::Expr {
        self
    }
}

/// Bridge between a Rust method name and its JS form. Implementers list the
/// methods they support; everything else panics at JS codegen time —
/// `ExprMethodCall::to_js` adds this trait as a bound so unsupported receiver
/// types fail to compile rather than silently producing broken JS.
///
/// The receiver expression has already been emitted (wrapped in parens) when
/// `js_call` is invoked; impls append the JS suffix for the method (e.g.
/// `.toLowerCase()`, `.length`, or nothing at all for a no-op like `.clone()`
/// on a value-typed primitive).
pub trait JsCallable {
    fn js_call(method: &str, out: &mut String);
}

/// Method calls on `&T` dispatch to `T`'s impl — keeps the surface narrow so
/// per-type impls only need to consider the owned form.
impl<T: JsCallable> JsCallable for &T {
    fn js_call(method: &str, out: &mut String) {
        T::js_call(method, out);
    }
}
