use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::{Arc, Mutex},
};

use bit_set::BitSet;

use super::ErasedBinding;

pub(super) struct Binding<T> {
    pub(super) id: Id,
    pub(super) value: T,
}

impl<T> Binding<T>
where
    T: Any + Send + Sync,
{
    pub(super) fn erase(id: Id, value: T) -> ErasedBinding {
        Arc::new(Self { id, value })
    }

    pub(super) fn downcast(binding: &ErasedBinding) -> &Self {
        binding
            .downcast_ref()
            .expect("context binding type changed")
    }

    pub(super) fn downcast_unique(binding: &mut ErasedBinding) -> &mut Self {
        Arc::get_mut(binding)
            .unwrap_or_else(|| panic!("request root binding is still shared with a scoped context"))
            .downcast_mut()
            .expect("context binding type changed")
    }

    pub(super) fn into_value(binding: ErasedBinding) -> T {
        let binding = binding
            .downcast::<Self>()
            .unwrap_or_else(|_| panic!("context binding type changed"));
        Arc::try_unwrap(binding)
            .unwrap_or_else(|_| {
                panic!("request root binding is still shared with a scoped context")
            })
            .value
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Id(pub(super) usize);

impl Id {
    pub(super) fn frontier(self) -> usize {
        self.0
            .checked_add(1)
            .expect("request context binding ID overflowed")
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct Mask {
    pub(super) bits: BitSet<usize>,
    pub(super) frontier: usize,
}

impl Mask {
    pub(super) fn install_root(
        &mut self,
        registry: &Registry,
        type_id: TypeId,
        previous_id: Option<Id>,
    ) -> Id {
        let (first_id, binding_id) = registry.allocate_root_value(type_id);
        self.advance(registry, binding_id.frontier());
        if let Some(previous_id) = previous_id.or(first_id) {
            let removed = self.bits.remove(previous_id.0);
            debug_assert!(removed, "previous context binding was not visible");
        }
        self.bits.insert(binding_id.0);
        binding_id
    }

    pub(super) fn install_scoped(
        &mut self,
        registry: &Registry,
        type_id: TypeId,
        previous_id: Option<Id>,
    ) -> Id {
        let (root_none_id, binding_id) = registry.allocate_scoped_value(type_id);
        self.advance(registry, binding_id.frontier());
        let previous_id = previous_id.unwrap_or(root_none_id);
        let removed = self.bits.remove(previous_id.0);
        debug_assert!(removed, "previous context binding was not visible");
        self.bits.insert(binding_id.0);
        binding_id
    }

    fn advance(&mut self, registry: &Registry, frontier: usize) {
        assert!(
            frontier >= self.frontier,
            "request context binding mask cannot move backwards"
        );
        registry.materialize_first(&mut self.bits, self.frontier, frontier);
        self.frontier = frontier;
    }

    pub(super) fn effectively_contains(&self, registry: &Registry, binding_id: Id) -> bool {
        if binding_id.0 < self.frontier {
            self.bits.contains(binding_id.0)
        } else {
            registry.is_first(binding_id)
        }
    }
}

#[derive(Debug, Default)]
struct RegistryState {
    // A first ID is either a root value or the implicit None that precedes a
    // first scoped value. No binding mask can predate a first root value.
    first: HashMap<TypeId, Id>,
    next_id: usize,
}

impl RegistryState {
    fn allocate(&mut self) -> Id {
        let id = Id(self.next_id);
        self.next_id = self
            .next_id
            .checked_add(1)
            .expect("request context binding ID overflowed");
        id
    }

    fn first_or_allocate(&mut self, type_id: TypeId) -> (Id, bool) {
        if let Some(&first_id) = self.first.get(&type_id) {
            return (first_id, false);
        }

        let first_id = self.allocate();
        self.first.insert(type_id, first_id);
        (first_id, true)
    }
}

#[derive(Debug, Default)]
pub(super) struct Registry {
    state: Mutex<RegistryState>,
}

impl Registry {
    pub(super) fn root_none(&self, type_id: TypeId) -> Id {
        self.state.lock().unwrap().first_or_allocate(type_id).0
    }

    fn allocate_root_value(&self, type_id: TypeId) -> (Option<Id>, Id) {
        let mut state = self.state.lock().unwrap();
        let (first_id, is_new) = state.first_or_allocate(type_id);
        if is_new {
            (None, first_id)
        } else {
            (Some(first_id), state.allocate())
        }
    }

    fn allocate_scoped_value(&self, type_id: TypeId) -> (Id, Id) {
        let mut state = self.state.lock().unwrap();
        let first_id = state.first_or_allocate(type_id).0;
        (first_id, state.allocate())
    }

    fn materialize_first(&self, bits: &mut BitSet<usize>, start: usize, end: usize) {
        let state = self.state.lock().unwrap();
        for first_id in state.first.values() {
            if (start..end).contains(&first_id.0) {
                bits.insert(first_id.0);
            }
        }
    }

    fn is_first(&self, binding_id: Id) -> bool {
        self.state
            .lock()
            .unwrap()
            .first
            .values()
            .any(|&first_id| first_id == binding_id)
    }
}
