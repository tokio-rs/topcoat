use std::marker::PhantomData;

/// References a closure parameter by name. The user-annotated parameter type
/// flows in as `T`, so field accesses against this expression resolve against
/// the real type. Server-side `eval` is unreachable — handlers do not run
/// during SSR.
pub struct ExprParam<T> {
    name: &'static str,
    _phantom: PhantomData<fn() -> T>,
}

impl<T> ExprParam<T> {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            _phantom: PhantomData,
        }
    }

    pub fn name(&self) -> &'static str {
        self.name
    }
}

impl<T> Clone for ExprParam<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for ExprParam<T> {}
