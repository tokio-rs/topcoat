//! Values kept alive for the rest of a request.

use std::{
    any::{Any, TypeId},
    fmt,
};

use elsa::sync::{FrozenMap, FrozenVec};

/// The per-request store for values that live until the request ends.
///
/// A value put in the arena keeps a stable address for the rest of the
/// request, so the arena hands out references tied to the request rather
/// than to the scope that created the value. Nothing in the arena can be
/// taken out or replaced; everything is dropped with the request.
///
/// Besides the anonymous values [`alloc`](Self::alloc) stores, the arena
/// holds one slot per type, created on first access through
/// [`get_or_default`](Self::get_or_default), for state a feature keeps
/// across a request.
#[derive(Default)]
#[doc(hidden)]
pub struct RequestArena {
    /// Values are boxed so their addresses stay stable while the vector
    /// grows.
    values: FrozenVec<Box<dyn Any + Send + Sync>>,
    /// One value per type, boxed for the same reason.
    slots: FrozenMap<TypeId, Box<dyn Any + Send + Sync>>,
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
        Self::downcast(self.values.push_get(Box::new(value)))
    }

    /// Returns the request's value of type `T`, creating it from its default
    /// on first access.
    ///
    /// Every call within one request returns the same value, so a type can
    /// act as a per-request singleton; interior mutability is needed to
    /// change it.
    pub fn get_or_default<T>(&self) -> &T
    where
        T: Any + Default + Send + Sync,
    {
        let type_id = TypeId::of::<T>();
        let slot = match self.slots.get(&type_id) {
            Some(slot) => slot,
            None => self.slots.insert_with(type_id, || Box::new(T::default())),
        };
        Self::downcast(slot)
    }

    /// Recovers the concrete type of a value the arena stores; the type
    /// cannot change between storing and reading a value.
    fn downcast<T>(value: &(dyn Any + Send + Sync)) -> &T
    where
        T: Any,
    {
        value
            .downcast_ref()
            .expect("a value keeps its type in the arena")
    }
}

impl fmt::Debug for RequestArena {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RequestArena")
            .field("len", &self.values.len())
            .field("slots", &self.slots.len())
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

    #[test]
    fn get_or_default_returns_one_value_per_type() {
        #[derive(Default)]
        struct Counter(std::sync::atomic::AtomicU32);

        let arena = RequestArena::new();
        arena
            .get_or_default::<Counter>()
            .0
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        arena
            .get_or_default::<Counter>()
            .0
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let counter = arena.get_or_default::<Counter>();
        assert_eq!(counter.0.load(std::sync::atomic::Ordering::Relaxed), 2);
        assert!(std::ptr::eq(counter, arena.get_or_default::<Counter>()));
        assert_eq!(*arena.get_or_default::<u32>(), 0);
    }
}
