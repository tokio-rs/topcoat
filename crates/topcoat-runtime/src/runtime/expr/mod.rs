#[derive(Debug, Clone)]
pub struct Expr<T> {
    pub(crate) evaluated: T,
    pub(crate) js: String,
}

impl<T> Expr<T> {
    #[inline]
    pub fn new(evaluated: T, js: String) -> Self {
        Self { evaluated, js }
    }
}
