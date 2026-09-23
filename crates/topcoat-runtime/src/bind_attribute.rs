use topcoat_core::context::Cx;
use topcoat_view::{
    Attribute, AttributeKeyViewParts, AttributeValueViewParts, AttributeViewParts, PartsWriter,
    Unescaped,
};

use crate::Expr;

/// A bind attribute, written `:name=$(...)` in a `view!` body.
///
/// It renders the attribute with the expression's value from the server. If
/// the expression reads signals, it also renders the expression's JavaScript
/// into a `data-topcoat-bind:<name>` attribute, and the browser runtime keeps
/// the attribute in sync whenever those signals change. For `value`,
/// `checked`, `selected`, and `indeterminate`, the browser also sets the DOM
/// property of the same name.
#[derive(Debug, Clone)]
pub struct BindAttribute<K, V> {
    key: K,
    value: Expr<V>,
}

impl<K, V> BindAttribute<K, V> {
    /// Creates a binding of the attribute named by `key` to an expression.
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
        let Expr {
            evaluated,
            js,
            is_static,
        } = self.value;

        if is_static {
            Attribute::new(self.key, evaluated).into_view_parts(cx, parts);
            return;
        }

        Attribute::new(self.key.clone(), evaluated).into_view_parts(cx, parts);
        Attribute::new(
            (Unescaped::new_unchecked("data-topcoat-bind:"), self.key),
            js,
        )
        .into_view_parts(cx, parts);
    }
}
