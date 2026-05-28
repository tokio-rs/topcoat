use std::marker::PhantomData;

use crate::runtime::Expr;

/// A `receiver.method()` call. Only zero-argument methods are supported. The
/// accessor closure passed to `new` carries the real implementation so the
/// server-side `eval` can run it and rustc can resolve `T` from the receiver
/// type.
pub struct ExprMethodCall<R, F, T> {
    receiver: R,
    method: &'static str,
    accessor: F,
    _phantom: PhantomData<fn() -> T>,
}

impl<R, F, T> ExprMethodCall<R, F, T>
where
    R: Expr,
{
    pub fn new(receiver: R, method: &'static str, accessor: F) -> Self
    where
        F: FnOnce(R::Output) -> T,
    {
        Self {
            receiver,
            method,
            accessor,
            _phantom: PhantomData,
        }
    }
}
