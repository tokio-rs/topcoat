use crate::runtime::Expr;

#[derive(Debug, Clone)]
pub struct BindAttribute<K, V> {
    key: K,
    value: Expr<V>,
}

impl<K, V> BindAttribute<K, V> {
    #[inline]
    pub fn new(key: K, value: Expr<V>) -> Self {
        Self { key, value }
    }
}
