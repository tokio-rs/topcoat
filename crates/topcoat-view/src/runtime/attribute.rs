#[derive(Debug, Clone)]
pub struct Attribute<K, V> {
    key: K,
    value: V,
}

impl<K, V> Attribute<K, V> {
    #[inline]
    pub fn new(key: K, value: V) -> Self {
        Self { key, value }
    }
}
