use crate::{Expr, Interpreter, IntoExpr, Value};

pub struct ExprLit<T>(T);

impl<T> ExprLit<T> {
    pub fn new(inner: T) -> Self {
        Self(inner)
    }
}

impl<'a> Expr for ExprLit<&'a str> {
    type Output = &'a <str as Value>::Surrogate;

    fn eval(self, _interpreter: &mut Interpreter) -> Self::Output {
        self.0.ref_cast()
    }

    fn to_js(&self, out: &mut String) {
        let json = serde_json::to_string(&self.0).expect("literal is serializable as JSON");
        out.push_str(&json);
    }
}

macro_rules! impl_primitive {
    ($ty:ty) => {
        impl IntoExpr for $ty {
            type Expr = ExprLit<Self>;

            fn into_expr(self) -> Self::Expr {
                ExprLit::new(self)
            }
        }
    };
}

impl_primitive!(&'static str);
