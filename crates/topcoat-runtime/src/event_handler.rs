use topcoat_view::runtime::{IntoViewParts, Unescaped, ViewPart};

use crate::{Event, Expr, ExprClosure};

/// An event handler attribute. Emits a JavaScript closure expression into a
/// `data-topcoat-on:<event>` attribute on the element. The browser scanner
/// wraps it in `new Function('__context', …)` to obtain a real handler.
pub struct EventHandler<K, Body> {
    key: K,
    value: ExprClosure<(Event,), Body>,
}

impl<K, Body> EventHandler<K, Body> {
    #[inline]
    pub fn new(key: K, value: ExprClosure<(Event,), Body>) -> Self {
        Self { key, value }
    }
}

impl<K, Body> IntoViewParts for EventHandler<K, Body>
where
    K: IntoViewParts,
    Body: Expr,
{
    fn into_view_parts(self) -> impl Iterator<Item = ViewPart> {
        let mut js = String::new();
        self.value.to_js(&mut js);

        Unescaped::new_unchecked(" data-topcoat-on:")
            .into_view_parts()
            .chain(self.key.into_view_parts())
            .chain(Unescaped::new_unchecked("=\"").into_view_parts())
            .chain(js.into_view_parts())
            .chain(Unescaped::new_unchecked("\" ").into_view_parts())
    }
}
