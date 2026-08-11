use std::{
    any::Any,
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
};

use hashbrown::HashMap;

/// The per-request store backing deferred loads.
///
/// A deferred load is identified by the 128 bit identity of its call site,
/// derived by the caller from the component identity chain. The slot for one
/// identity is created on first access, holds "registered" and "resolved"
/// as separate states, and lives behind a stable address so `&V` references
/// handed out stay valid for the rest of the request.
#[derive(Debug, Default)]
#[doc(hidden)]
pub struct DeferCache {
    entries: Mutex<HashMap<u128, usize>>,
    values: boxcar::Vec<Box<dyn Any + Send + Sync + 'static>>,
}

impl DeferCache {
    #[must_use]
    pub fn new() -> Self {
        DeferCache::default()
    }

    /// Returns the slot for `identity`, creating an empty one on first
    /// access.
    ///
    /// # Panics
    ///
    /// Panics if the slot was created for a different value type, which
    /// means two deferred loads derived the same identity.
    #[track_caller]
    pub fn slot<V>(&self, identity: u128) -> &DeferSlot<V>
    where
        V: Send + Sync + 'static,
    {
        let index = {
            let mut entries = self.entries.lock().expect("defer cache lock poisoned");
            *entries
                .entry(identity)
                .or_insert_with(|| self.values.push(Box::<DeferSlot<V>>::default()))
        };
        self.values[index]
            .downcast_ref()
            .expect("two deferred loads with different value types derived the same identity")
    }
}

/// One deferred load's slot: whether its future has been registered, and the
/// output once it resolves.
#[derive(Debug)]
#[doc(hidden)]
pub struct DeferSlot<V> {
    registered: AtomicBool,
    value: OnceLock<V>,
}

impl<V> DeferSlot<V> {
    /// Marks the slot as registered. Returns whether this call was the one
    /// that registered it, so exactly one caller spawns the future.
    pub fn register(&self) -> bool {
        !self.registered.swap(true, Ordering::Relaxed)
    }

    /// The resolved output, if the deferred future has completed.
    pub fn get(&self) -> Option<&V> {
        self.value.get()
    }

    /// Stores the resolved output.
    ///
    /// # Panics
    ///
    /// Panics on a second resolution, which is a bug in the driver, not in
    /// application code.
    #[track_caller]
    pub fn resolve(&self, value: V) {
        assert!(
            self.value.set(value).is_ok(),
            "a deferred load resolved twice",
        );
    }
}

impl<V> Default for DeferSlot<V> {
    fn default() -> Self {
        DeferSlot {
            registered: AtomicBool::new(false),
            value: OnceLock::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_is_created_once_and_keeps_its_address() {
        let cache = DeferCache::new();
        let first: *const DeferSlot<u32> = cache.slot::<u32>(7);
        for _ in 0..64 {
            cache.slot::<u32>(99);
        }
        let again: *const DeferSlot<u32> = cache.slot::<u32>(7);
        assert_eq!(first, again);
    }

    #[test]
    fn register_elects_exactly_one_caller() {
        let slot = DeferSlot::<u32>::default();
        assert!(slot.register());
        assert!(!slot.register());
    }

    #[test]
    fn resolve_makes_the_value_visible() {
        let slot = DeferSlot::<String>::default();
        assert_eq!(slot.get(), None);
        slot.resolve(String::from("ready"));
        assert_eq!(slot.get().map(String::as_str), Some("ready"));
    }

    #[test]
    #[should_panic(expected = "resolved twice")]
    fn double_resolution_panics() {
        let slot = DeferSlot::<u32>::default();
        slot.resolve(1);
        slot.resolve(2);
    }

    #[test]
    #[should_panic(expected = "different value types")]
    fn identity_collision_across_types_panics() {
        let cache = DeferCache::new();
        cache.slot::<u32>(1);
        cache.slot::<String>(1);
    }
}
