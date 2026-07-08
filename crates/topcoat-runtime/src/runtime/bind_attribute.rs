use topcoat_core::runtime::context::Cx;
use topcoat_view::runtime::{
    AttributeKeyViewParts, AttributeValueViewParts, AttributeViewParts, Unescaped, ViewParts,
};

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

impl<K, V> AttributeViewParts for BindAttribute<K, V>
where
    K: AttributeKeyViewParts + Clone,
    V: AttributeValueViewParts,
{
    #[inline]
    fn into_view_parts(self, cx: &Cx, parts: &mut ViewParts) {
        let Expr { evaluated, js } = self.value;

        parts.push(Unescaped::new_unchecked(" "));
        self.key.clone().into_view_parts(cx, parts);
        parts.push(Unescaped::new_unchecked("=\""));
        evaluated.into_view_parts(cx, parts);
        parts.push(Unescaped::new_unchecked("\" data-topcoat-bind:"));
        self.key.into_view_parts(cx, parts);
        parts.push(Unescaped::new_unchecked("=\""));
        parts.push(js);
        parts.push(Unescaped::new_unchecked("\" "));
    }
}
