use topcoat_core::context::Cx;
use topcoat_view::{Attribute, AttributeKeyViewParts, AttributeViewParts, PartsWriter, Unescaped};

use crate::{Event, Expr};

/// A closure that can handle a DOM event: any `FnOnce` taking an [`Event`].
///
/// Implemented automatically, so there is nothing to implement by hand.
pub trait EventHandlerFn {}

impl<T, R> EventHandlerFn for T where T: FnOnce(Event) -> R {}

/// An event handler attribute, written `@event=$(...)` in a `view!` body.
///
/// It renders the JavaScript of its closure into a `data-topcoat-on:<event>`
/// attribute. The browser runtime turns that source into a listener for the
/// event on the element. The closure itself never runs on the server.
pub struct EventHandler<K, F> {
    key: K,
    value: Expr<F>,
}

impl<K, F> EventHandler<K, F>
where
    F: EventHandlerFn,
{
    /// Creates a handler for the event named by `key` from a closure
    /// expression.
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
