use topcoat_core::runtime::context::Cx;
use topcoat_view::runtime::{
    Attribute, AttributeKeyViewParts, AttributeValueViewParts, AttributeViewParts, PartsWriter,
    Unescaped,
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
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        let Expr { evaluated, js } = self.value;

        Attribute::new(self.key.clone(), evaluated).into_view_parts(cx, parts);
        Attribute::new(
            (Unescaped::new_unchecked("data-topcoat-bind:"), self.key),
            js,
        )
        .into_view_parts(cx, parts);
    }
}
