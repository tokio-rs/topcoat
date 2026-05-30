use topcoat_view::runtime::{IntoViewParts, Unescaped, ViewPart};

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

impl<K, V> IntoViewParts for BindAttribute<K, V>
where
    K: IntoViewParts + Clone,
    V: IntoViewParts,
{
    fn into_view_parts(self) -> impl Iterator<Item = ViewPart> {
        Unescaped::new_unchecked(" ")
            .into_view_parts()
            .chain(self.key.clone().into_view_parts())
            .chain(Unescaped::new_unchecked("=\"").into_view_parts())
            .chain(self.value.evaluated.into_view_parts())
            .chain(Unescaped::new_unchecked("\" data-topcoat-bind:").into_view_parts())
            .chain(self.key.into_view_parts())
            .chain(Unescaped::new_unchecked("=\"").into_view_parts())
            .chain(self.value.js.into_view_parts())
            .chain(Unescaped::new_unchecked("\" ").into_view_parts())
    }
}
