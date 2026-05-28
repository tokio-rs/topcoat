use std::marker::PhantomData;

use crate::runtime::{Eval, Expr, ExprParam, FmtJs, Formatter, Interpreter};

/// Per-arity storage for a closure's parameters. Each `ExprParam<T>` is just a
/// `&'static str` plus a zero-sized phantom, so the tuple is `Copy` and lives
/// inline in [`ExprClosure`] — no allocation.
pub trait ClosureParams {
    type Storage: Copy;
    fn write_param_list(storage: &Self::Storage, f: &mut Formatter<'_>);
}

macro_rules! impl_closure_params {
    ($( ($($idx:tt: $name:ident),*) ),* $(,)?) => {
        $(
            impl<$($name),*> ClosureParams for ($($name,)*) {
                type Storage = ($(ExprParam<$name>,)*);

                #[allow(unused_variables, unused_assignments)]
                fn write_param_list(storage: &Self::Storage, f: &mut Formatter<'_>) {
                    #[allow(unused_mut)]
                    let mut first = true;
                    $(
                        if !first { f.write_str(", "); }
                        f.write_str(storage.$idx.name());
                        first = false;
                    )*
                }
            }
        )*
    };
}

impl_closure_params! {
    (),
    (0: T1),
    (0: T1, 1: T2),
    (0: T1, 1: T2, 2: T3),
    (0: T1, 1: T2, 2: T3, 3: T4),
    (0: T1, 1: T2, 2: T3, 3: T4, 4: T5),
    (0: T1, 1: T2, 2: T3, 3: T4, 4: T5, 5: T6),
    (0: T1, 1: T2, 2: T3, 3: T4, 4: T5, 5: T6, 6: T7),
    (0: T1, 1: T2, 2: T3, 3: T4, 4: T5, 5: T6, 6: T7, 7: T8),
}

pub struct ExprClosure<Params, Body>
where
    Params: ClosureParams,
{
    storage: Params::Storage,
    body: Body,
    _phantom: PhantomData<fn(Params) -> ()>,
}

macro_rules! impl_new {
    ($( ($($idx:tt: $name:ident),*) ),* $(,)?) => {
        $(
            impl<$($name,)* Body> ExprClosure<($($name,)*), Body> {
                #[allow(clippy::too_many_arguments)]
                pub fn new(
                    params: ($(&ExprParam<$name>,)*),
                    body: Body,
                ) -> Self {
                    Self {
                        storage: ($(*params.$idx,)*),
                        body,
                        _phantom: PhantomData,
                    }
                }
            }
        )*
    };
}

impl<Body> ExprClosure<(), Body> {
    pub fn new(_params: (), body: Body) -> Self {
        Self {
            storage: (),
            body,
            _phantom: PhantomData,
        }
    }
}

impl_new! {
    (0: T1),
    (0: T1, 1: T2),
    (0: T1, 1: T2, 2: T3),
    (0: T1, 1: T2, 2: T3, 3: T4),
    (0: T1, 1: T2, 2: T3, 3: T4, 4: T5),
    (0: T1, 1: T2, 2: T3, 3: T4, 4: T5, 5: T6),
    (0: T1, 1: T2, 2: T3, 3: T4, 4: T5, 5: T6, 6: T7),
    (0: T1, 1: T2, 2: T3, 3: T4, 4: T5, 5: T6, 6: T7, 7: T8),
}

impl<Params, Body> Eval for ExprClosure<Params, Body>
where
    Params: ClosureParams,
    Body: Eval,
{
    type Output = Body::Output;

    fn eval(self, _interpreter: &mut Interpreter) -> Self::Output {
        unreachable!("ExprClosure::eval called server-side; handlers do not run during SSR")
    }
}

impl<Params, Body> FmtJs for ExprClosure<Params, Body>
where
    Params: ClosureParams,
    Body: FmtJs,
{
    fn fmt_js(&self, f: &mut Formatter<'_>) {
        f.write_str("((");
        Params::write_param_list(&self.storage, f);
        f.write_str(") => { ");
        self.body.fmt_js(f);
        f.write_str("; })");
    }
}
