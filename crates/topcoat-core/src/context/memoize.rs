use std::{
    any::{Any, TypeId},
    collections::HashMap,
    hash::{Hash, Hasher},
    sync::{Arc, Mutex},
};

pub(super) struct DynRequestCache {
    entries: Mutex<HashMap<Box<dyn DynKey>, Arc<dyn Any + Send + Sync>>>,
}

impl DynRequestCache {
    pub(super) fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
        }
    }

    fn get<T: Send + Sync + 'static>(&self, key: &dyn DynKey) -> Option<Arc<T>> {
        let value = {
            let guard = self.entries.lock().unwrap();
            guard.get(key)?.clone()
        };
        Some(value.downcast::<T>().unwrap())
    }

    fn insert<T: Send + Sync + 'static>(&mut self, key: Box<dyn DynKey>, value: T) {
        let mut guard = self.entries.lock().unwrap();
        guard.insert(key, Arc::new(value));
    }
}

impl std::fmt::Debug for DynRequestCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DynRequestCache").finish()
    }
}

trait DynKey: Any + Send + Sync {
    fn dyn_eq(&self, other: &dyn DynKey) -> bool;
    fn dyn_hash(&self, state: &mut dyn Hasher);
    fn as_any(&self) -> &dyn Any;
}

impl<T: Any + Eq + Hash + Send + Sync> DynKey for T {
    fn dyn_eq(&self, other: &dyn DynKey) -> bool {
        other.as_any().downcast_ref::<T>() == Some(self)
    }

    fn dyn_hash(&self, mut state: &mut dyn Hasher) {
        TypeId::of::<T>().hash(&mut state);
        self.hash(&mut state);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl PartialEq for dyn DynKey {
    fn eq(&self, other: &Self) -> bool {
        self.dyn_eq(other)
    }
}

impl Eq for dyn DynKey {}

impl Hash for dyn DynKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.dyn_hash(state);
    }
}
