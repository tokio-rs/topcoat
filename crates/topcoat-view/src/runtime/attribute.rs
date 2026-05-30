use crate::runtime::{IntoViewParts, Unescaped, ViewPart};

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

impl<K, V> IntoViewParts for Attribute<K, V>
where
    K: IntoViewParts,
    V: IntoViewParts,
{
    fn into_view_parts(self) -> impl Iterator<Item = ViewPart> {
        Unescaped::new_unchecked(" ")
            .into_view_parts()
            .chain(self.key.into_view_parts())
            .chain(Unescaped::new_unchecked("=\"").into_view_parts())
            .chain(self.value.into_view_parts())
            .chain(Unescaped::new_unchecked("\"").into_view_parts())
    }
}
