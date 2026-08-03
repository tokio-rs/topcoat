use std::{
    any::{Any, TypeId},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

type ErasedValue = Arc<dyn Any + Send + Sync>;

#[derive(Clone, Default)]
pub(super) struct BindingSet {
    entries: im::HashMap<TypeId, Binding>,
}

impl BindingSet {
    pub(super) fn install<T>(&mut self, ids: &IdAllocator, value: T) -> Option<Binding>
    where
        T: Any + Send + Sync,
    {
        self.entries
            .insert(TypeId::of::<T>(), Binding::new(ids.allocate(), value))
    }

    pub(super) fn get<T>(&self) -> Option<&Binding>
    where
        T: Any + Send + Sync,
    {
        self.entries.get(&TypeId::of::<T>())
    }

    pub(super) fn get_mut<T>(&mut self, ids: &IdAllocator) -> Option<&mut T>
    where
        T: Any + Send + Sync,
    {
        let Binding { id, value } = self.entries.get_mut(&TypeId::of::<T>())?;
        let value = Arc::get_mut(value)
            .unwrap_or_else(|| panic!("request root binding is still shared with a scoped context"))
            .downcast_mut()
            .expect("context binding type changed");
        *id = ids.allocate();
        Some(value)
    }

    pub(super) fn resolve(&self, type_id: TypeId) -> Option<Id> {
        self.entries.get(&type_id).map(|binding| binding.id)
    }
}

#[derive(Clone)]
pub(super) struct Binding {
    pub(super) id: Id,
    value: ErasedValue,
}

impl Binding {
    fn new<T>(id: Id, value: T) -> Self
    where
        T: Any + Send + Sync,
    {
        Self {
            id,
            value: Arc::new(value),
        }
    }

    pub(super) fn value<T>(&self) -> &T
    where
        T: Any + Send + Sync,
    {
        self.value
            .downcast_ref()
            .expect("context binding type changed")
    }

    pub(super) fn into_value<T>(self) -> T
    where
        T: Any + Send + Sync,
    {
        let value = self
            .value
            .downcast::<T>()
            .unwrap_or_else(|_| panic!("context binding type changed"));
        Arc::try_unwrap(value).unwrap_or_else(|_| {
            panic!("request root binding is still shared with a scoped context")
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Id(usize);

#[derive(Debug, Default)]
pub(super) struct IdAllocator(AtomicUsize);

impl IdAllocator {
    fn allocate(&self) -> Id {
        let id = self
            .0
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
            .unwrap_or_else(|_| panic!("request context binding ID overflowed"));
        Id(id)
    }
}
