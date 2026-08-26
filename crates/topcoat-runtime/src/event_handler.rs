use topcoat_core::context::Cx;
use topcoat_view::{Attribute, AttributeKeyViewParts, AttributeViewParts, PartsWriter, Unescaped};

use crate::{Event, Expr};

pub trait EventHandlerFn {}

impl<T, R> EventHandlerFn for T where T: Fn(Event) -> R {}

/// An event handler attribute. Emits a JavaScript closure expression into a
/// `data-topcoat-on:<event>` attribute on the element. The browser scanner
/// wraps it in `new Function('__cx', ...)` to obtain a real handler.
pub struct EventHandler<K, F> {
    key: K,
    value: Expr<F>,
}

impl<K, F> EventHandler<K, F>
where
    F: EventHandlerFn,
{
    #[inline]
    pub fn new(key: K, value: Expr<F>) -> Self {
        Self { key, value }
    }
}

impl<K, F> AttributeViewParts for EventHandler<K, F>
where
    K: AttributeKeyViewParts,
{
    #[inline]
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        Attribute::new(
            (Unescaped::new_unchecked("data-topcoat-on:"), self.key),
            self.value.js,
        )
        .into_view_parts(cx, parts);
    }
}
