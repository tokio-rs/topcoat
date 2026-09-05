//! Values kept alive for the rest of a request.

use std::{any::Any, fmt};

use elsa::sync::FrozenVec;

/// The per-request store for values that live until the request ends.
///
/// A value put in the arena keeps a stable address for the rest of the
/// request, so the arena hands out references tied to the request rather
/// than to the scope that created the value. Nothing in the arena can be
/// taken out or replaced; everything is dropped with the request.
#[derive(Default)]
#[doc(hidden)]
pub struct RequestArena {
    /// Values are boxed so their addresses stay stable while the vector
    /// grows.
    values: FrozenVec<Box<dyn Any + Send + Sync>>,
}

impl RequestArena {
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Stores `value` for the rest of the request and returns a reference to
    /// it.
    pub fn alloc<T>(&self, value: T) -> &T
    where
        T: Any + Send + Sync,
    {
        self.values
            .push_get(Box::new(value))
            .downcast_ref()
            .expect("a value keeps its type in the arena")
    }
}

impl fmt::Debug for RequestArena {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RequestArena")
            .field("len", &self.values.len())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alloc_returns_the_stored_value() {
        let arena = RequestArena::new();
        assert_eq!(arena.alloc(String::from("a")), "a");
        assert_eq!(*arena.alloc(7u32), 7);
    }

    #[test]
    fn references_stay_valid_while_the_arena_grows() {
        let arena = RequestArena::new();
        let first = arena.alloc(String::from("first"));
        let address = std::ptr::from_ref(first);
        for i in 0..1000 {
            arena.alloc(i);
        }
        assert_eq!(first, "first");
        assert_eq!(std::ptr::from_ref(first), address);
    }
}
