use std::marker::PhantomData;

use crate::{Expr, Interpreter};

pub struct ExprClosure<Params, Body> {
    params: &'static [&'static str],
    body: Body,
    _phantom: PhantomData<fn(Params) -> ()>,
}

impl<Params, Body> ExprClosure<Params, Body> {
    pub fn new(params: &'static [&'static str], body: Body) -> Self {
        Self {
            params,
            body,
            _phantom: PhantomData,
        }
    }
}

impl<Params, Body> Expr for ExprClosure<Params, Body>
where
    Body: Expr,
{
    type Output = Body::Output;

    fn eval(self, _interpreter: &mut Interpreter) -> Self::Output {
        unreachable!("ExprClosure::eval called server-side; handlers do not run during SSR")
    }

    fn to_js(&self, out: &mut String) {
        out.push_str("((");
        for (i, name) in self.params.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            out.push_str(name);
        }
        out.push_str(") => { ");
        self.body.to_js(out);
        out.push_str("; })");
    }
}
