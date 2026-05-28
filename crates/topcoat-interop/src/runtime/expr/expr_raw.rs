use std::marker::PhantomData;

use crate::runtime::{Eval, FmtJs, Interpreter};

pub struct ExprRaw<T> {
    js: &'static [&'static str],
    slots: Vec<Box<dyn FmtJs>>,
    eval_as: Option<T>,
    _phantom: PhantomData<T>,
}

impl<T> ExprRaw<T> {
    pub fn new(js: &'static [&'static str], slots: Vec<Box<dyn FmtJs>>) -> Self {
        assert_eq!(js.len(), slots.len() - 1);
        Self {
            js,
            slots,
            eval_as: None,
            _phantom: PhantomData,
        }
    }

    pub fn eval_as(mut self, eval_as: T) -> Self {
        self.eval_as = Some(eval_as);
        self
    }
}

impl<T> FmtJs for ExprRaw<T> {
    fn fmt_js(&self, f: &mut super::Formatter<'_>) {
        for i in 0..self.js.len() {
            f.write_str(self.js[i]);
            if i < self.js.len() - 1 {
                self.slots[i].fmt_js(f);
            }
        }
    }
}

impl<T> Eval for ExprRaw<T> {
    type Output = T;

    fn eval(self, _interpreter: &mut Interpreter) -> Self::Output {
        match self.eval_as {
            Some(value) => value,
            None => panic!(
                "raw expression can only be evaluated if a rust value is provided, use `.eval_as(value)`"
            ),
        }
    }
}
