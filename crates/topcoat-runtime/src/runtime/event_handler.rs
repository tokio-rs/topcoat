use topcoat_view::runtime::{IntoViewParts, Unescaped, ViewPart};

use crate::runtime::{Event, Expr};

/// An event handler attribute. Emits a JavaScript closure expression into a
/// `data-topcoat-on:<event>` attribute on the element. The browser scanner
/// wraps it in `new Function('__context', …)` to obtain a real handler.
pub struct EventHandler<K, F> {
    key: K,
    value: Expr<F>,
}

impl<K, F> EventHandler<K, F>
where
    F: Fn(Event),
{
    #[inline]
    pub fn new(key: K, value: Expr<F>) -> Self {
        Self { key, value }
    }
}

impl<K, F> IntoViewParts for EventHandler<K, F>
where
    K: IntoViewParts + Clone,
{
    fn into_view_parts(self) -> impl Iterator<Item = ViewPart> {
        Unescaped::new_unchecked(" data-topcoat-on:")
            .into_view_parts()
            .chain(self.key.into_view_parts())
            .chain(Unescaped::new_unchecked("=\"").into_view_parts())
            .chain(self.value.js.into_view_parts())
            .chain(Unescaped::new_unchecked("\" ").into_view_parts())
    }
}
