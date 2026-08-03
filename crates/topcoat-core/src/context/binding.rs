use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::Arc,
};

use bit_set::BitSet;

type ErasedBinding = Arc<dyn Any + Send + Sync>;

#[derive(Clone, Default)]
pub(super) struct BindingSet {
    entries: im::HashMap<TypeId, ErasedBinding>,
    pub(super) mask: Mask,
}

impl BindingSet {
    pub(super) fn install_root<T>(&mut self, registry: &mut Registry, value: T) -> Option<T>
    where
        T: Any + Send + Sync,
    {
        let type_id = TypeId::of::<T>();
        let previous_id = self.get::<T>().map(|binding| binding.id);
        let binding_id = self.mask.install_root(registry, type_id, previous_id);
        self.entries
            .insert(type_id, Binding::erase(binding_id, value))
            .map(Binding::<T>::into_value)
    }

    pub(super) fn install_scoped<T>(&mut self, registry: &mut Registry, value: T)
    where
        T: Any + Send + Sync,
    {
        let type_id = TypeId::of::<T>();
        let previous_id = self.get::<T>().map(|binding| binding.id);
        let binding_id = self.mask.install_scoped(registry, type_id, previous_id);
        self.entries
            .insert(type_id, Binding::erase(binding_id, value));
    }

    pub(super) fn get<T>(&self) -> Option<&Binding<T>>
    where
        T: Any + Send + Sync,
    {
        self.entries.get(&TypeId::of::<T>()).map(Binding::downcast)
    }

    pub(super) fn get_mut<T>(&mut self, registry: &mut Registry) -> Option<&mut T>
    where
        T: Any + Send + Sync,
    {
        let type_id = TypeId::of::<T>();
        let binding = Binding::<T>::downcast_unique(self.entries.get_mut(&type_id)?);
        binding.id = self.mask.install_root(registry, type_id, Some(binding.id));
        Some(&mut binding.value)
    }
}

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
        registry: &mut Registry,
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
        registry: &mut Registry,
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
        registry.materialize_root_none(&mut self.bits, self.frontier, frontier);
        self.frontier = frontier;
    }

    pub(super) fn effectively_contains(&self, registry: &Registry, binding_id: Id) -> bool {
        if binding_id.0 < self.frontier {
            self.bits.contains(binding_id.0)
        } else {
            registry.is_root_none(binding_id)
        }
    }

    pub(super) fn union_visible_reads(
        &self,
        tracked: &mut BitSet<usize>,
        reads: &BitSet<usize>,
        registry: &Registry,
    ) {
        tracked.extend(reads.intersection(&self.bits));
        registry.union_root_none_reads(tracked, reads, self.frontier);
    }

    pub(super) fn contains_reads(&self, reads: &BitSet<usize>, registry: &Registry) -> bool {
        registry.contains_root_none_reads(reads.difference(&self.bits), self.frontier)
    }
}

#[derive(Debug, Default)]
pub(super) struct Registry {
    // No binding mask can predate a first root value. A mask can predate an
    // implicit root None, so those IDs are tracked separately.
    first: HashMap<TypeId, Id>,
    root_none: BitSet<usize>,
    next_id: usize,
}

impl Registry {
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

    fn first_or_allocate_root_none(&mut self, type_id: TypeId) -> Id {
        let (root_none_id, is_new) = self.first_or_allocate(type_id);
        if is_new {
            let inserted = self.root_none.insert(root_none_id.0);
            debug_assert!(inserted, "root None binding ID was already registered");
        }
        root_none_id
    }

    pub(super) fn root_none(&mut self, type_id: TypeId) -> Id {
        self.first_or_allocate_root_none(type_id)
    }

    fn allocate_root_value(&mut self, type_id: TypeId) -> (Option<Id>, Id) {
        let (first_id, is_new) = self.first_or_allocate(type_id);
        if is_new {
            (None, first_id)
        } else {
            (Some(first_id), self.allocate())
        }
    }

    fn allocate_scoped_value(&mut self, type_id: TypeId) -> (Id, Id) {
        let root_none_id = self.first_or_allocate_root_none(type_id);
        (root_none_id, self.allocate())
    }

    fn materialize_root_none(&self, bits: &mut BitSet<usize>, start: usize, end: usize) {
        for index in self
            .root_none
            .iter()
            .skip_while(|&index| index < start)
            .take_while(|&index| index < end)
        {
            bits.insert(index);
        }
    }

    fn is_root_none(&self, binding_id: Id) -> bool {
        self.root_none.contains(binding_id.0)
    }

    fn union_root_none_reads(
        &self,
        tracked: &mut BitSet<usize>,
        reads: &BitSet<usize>,
        frontier: usize,
    ) {
        for index in reads.iter().skip_while(|&index| index < frontier) {
            if self.root_none.contains(index) {
                tracked.insert(index);
            }
        }
    }

    fn contains_root_none_reads(
        &self,
        mut reads: impl Iterator<Item = usize>,
        frontier: usize,
    ) -> bool {
        reads.all(|index| index >= frontier && self.root_none.contains(index))
    }
}
