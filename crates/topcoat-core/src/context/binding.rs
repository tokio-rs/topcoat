use std::{
    any::{Any, TypeId},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

#[derive(Debug, Default)]
pub(super) struct RootBindings {
    entries: anymap3::Map<dyn Any + Send + Sync>,
}

impl RootBindings {
    pub(super) fn install<T>(&mut self, ids: &IdAllocator, value: T) -> (Option<T>, Id)
    where
        T: Any + Send + Sync,
    {
        let id = ids.allocate();
        let previous = self
            .entries
            .insert(RootBinding { id, value })
            .map(|binding| binding.value);
        (previous, id)
    }

    #[inline]
    pub(super) fn get<T>(&self) -> Option<&RootBinding<T>>
    where
        T: Any + Send + Sync,
    {
        self.entries.get()
    }

    pub(super) fn get_mut<T>(&mut self) -> Option<(&mut T, &mut Id)>
    where
        T: Any + Send + Sync,
    {
        let binding = self.entries.get_mut::<RootBinding<T>>()?;
        Some((&mut binding.value, &mut binding.id))
    }
}

pub(super) struct RootBinding<T> {
    pub(super) id: Id,
    pub(super) value: T,
}

type ErasedValue = Arc<dyn Any + Send + Sync>;

#[derive(Clone, Default)]
pub(super) struct ScopedBindings {
    entries: im::HashMap<TypeId, ScopedBinding>,
}

impl ScopedBindings {
    pub(super) fn install<T>(&mut self, ids: &IdAllocator, value: T) -> Id
    where
        T: Any + Send + Sync,
    {
        let id = ids.allocate();
        self.entries.insert(
            TypeId::of::<T>(),
            ScopedBinding {
                id,
                value: Arc::new(value),
            },
        );
        id
    }

    #[inline]
    pub(super) fn get<T>(&self) -> Option<&ScopedBinding>
    where
        T: Any + Send + Sync,
    {
        self.entries.get(&TypeId::of::<T>())
    }
}

pub(super) struct ScopedBinding {
    pub(super) id: Id,
    value: ErasedValue,
}

impl ScopedBinding {
    #[inline]
    pub(super) fn value<T>(&self) -> &T
    where
        T: Any + Send + Sync,
    {
        self.value
            .downcast_ref()
            .expect("context binding type changed")
    }
}

impl Clone for ScopedBinding {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            value: self.value.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Id(usize);

impl Id {
    pub(super) fn get(self) -> usize {
        self.0
    }
}

#[derive(Debug, Default)]
pub(super) struct IdAllocator(AtomicUsize);

impl IdAllocator {
    pub(super) fn allocate(&self) -> Id {
        let id = self
            .0
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
            .unwrap_or_else(|_| panic!("request context binding ID overflowed"));
        Id(id)
    }
}
