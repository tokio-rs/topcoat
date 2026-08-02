use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::{Arc, Mutex},
};

use bit_set::BitSet;

use super::ContextValue;

pub(super) struct Context {
    pub(super) id: Id,
    pub(super) value: ContextValue,
}

impl Context {
    pub(super) fn into_value<T>(binding: Arc<Self>) -> T
    where
        T: Any + Send + Sync,
    {
        let binding = Arc::try_unwrap(binding).unwrap_or_else(|_| {
            panic!("request root binding is still shared with a scoped context")
        });
        *binding
            .value
            .downcast::<T>()
            .expect("context binding type changed")
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

    #[cfg(test)]
    fn count(&self) -> usize {
        self.state.lock().unwrap().next_id
    }
}

#[cfg(test)]
mod tests {
    use std::{any::TypeId, sync::Barrier};

    use super::*;
    use crate::context::{Cx, CxTestBuilder, request_context, try_request_context};

    #[derive(Debug, PartialEq)]
    struct Database(&'static str);

    #[derive(Debug, PartialEq)]
    struct Config(u32);

    #[test]
    fn concurrent_missing_reads_share_one_root_none_identity() {
        let cx = Cx::default();
        let ready = Barrier::new(3);

        let (first_id, second_id) = std::thread::scope(|scope| {
            let first = scope.spawn(|| {
                ready.wait();
                assert_eq!(try_request_context::<Database>(&cx), None);
                cx.resolve_binding_id(TypeId::of::<Database>())
            });
            let second = scope.spawn(|| {
                ready.wait();
                assert_eq!(try_request_context::<Database>(&cx), None);
                cx.resolve_binding_id(TypeId::of::<Database>())
            });
            ready.wait();
            (first.join().unwrap(), second.join().unwrap())
        });

        assert_eq!(first_id, second_id);
        assert_eq!(cx.request_state.bindings.count(), 1);
        assert!(cx.request_state.bindings.is_first(first_id));
    }

    #[test]
    fn contexts_created_before_root_none_treat_it_as_visible() {
        let cx = Cx::default();
        let before = cx.with(Config(1));
        let root_none_id = cx
            .request_state
            .bindings
            .root_none(TypeId::of::<Database>());

        assert!(root_none_id.0 >= before.binding_mask.frontier);
        assert!(!before.binding_mask.bits.contains(root_none_id.0));
        assert!(
            before
                .binding_mask
                .effectively_contains(&before.request_state.bindings, root_none_id)
        );
        assert_eq!(try_request_context::<Database>(&before), None);
    }

    #[test]
    fn first_scoped_value_allocates_and_shadows_root_none() {
        let cx = Cx::default();
        let child = cx.with(Database("primary"));
        let root_none_id = cx
            .request_state
            .bindings
            .root_none(TypeId::of::<Database>());
        let value_id = child.resolve_binding_id(TypeId::of::<Database>());

        assert_eq!(root_none_id, Id(0));
        assert_eq!(value_id, Id(1));
        assert!(
            cx.binding_mask
                .effectively_contains(&cx.request_state.bindings, root_none_id)
        );
        assert!(!child.binding_mask.bits.contains(root_none_id.0));
        assert!(child.binding_mask.bits.contains(value_id.0));
    }

    #[test]
    fn first_root_value_uses_the_first_binding_id() {
        let mut cx = Cx::default();

        assert_eq!(cx.insert(Database("primary")), None);
        let binding_id = cx.resolve_binding_id(TypeId::of::<Database>());

        assert_eq!(binding_id, Id(0));
        assert_eq!(cx.request_state.bindings.count(), 1);
        assert!(cx.request_state.bindings.is_first(binding_id));
        assert!(cx.binding_mask.bits.contains(binding_id.0));

        let child = cx.with(Config(1));
        assert_eq!(request_context::<Database>(&child), &Database("primary"));
        assert!(child.binding_mask.bits.contains(binding_id.0));
    }

    #[test]
    fn root_value_after_a_missing_read_shadows_root_none() {
        let mut cx = Cx::default();

        assert_eq!(try_request_context::<Database>(&cx), None);
        let root_none_id = cx.resolve_binding_id(TypeId::of::<Database>());
        assert_eq!(cx.insert(Database("primary")), None);
        let binding_id = cx.resolve_binding_id(TypeId::of::<Database>());

        assert_eq!(root_none_id, Id(0));
        assert_eq!(binding_id, Id(1));
        assert_eq!(cx.request_state.bindings.count(), 2);
        assert!(!cx.binding_mask.bits.contains(root_none_id.0));
        assert!(cx.binding_mask.bits.contains(binding_id.0));
    }

    #[test]
    fn advancing_for_an_unrelated_value_materializes_root_none() {
        let cx = Cx::default();
        let root_none_id = cx
            .request_state
            .bindings
            .root_none(TypeId::of::<Database>());

        assert!(!cx.binding_mask.bits.contains(root_none_id.0));
        let child = cx.with(Config(1));

        assert!(child.binding_mask.bits.contains(root_none_id.0));
        assert_eq!(try_request_context::<Database>(&child), None);
    }

    #[test]
    fn root_insert_replaces_and_returns_the_displaced_value() {
        let mut cx = Cx::default();

        assert_eq!(cx.insert(Database("primary")), None);
        let first_id = cx.resolve_binding_id(TypeId::of::<Database>());
        assert!(cx.binding_mask.bits.contains(first_id.0));
        assert_eq!(cx.insert(Database("replica")), Some(Database("primary")));
        let second_id = cx.resolve_binding_id(TypeId::of::<Database>());
        assert_eq!(request_context::<Database>(&cx), &Database("replica"));
        assert_ne!(first_id, second_id);
        assert!(!cx.binding_mask.bits.contains(first_id.0));
        assert!(cx.binding_mask.bits.contains(second_id.0));
    }

    #[test]
    fn root_first_value_advances_over_scoped_first_bindings() {
        let mut cx = Cx::default();
        let config_none_id;
        {
            let child = cx.with(Config(1));
            config_none_id = cx.request_state.bindings.root_none(TypeId::of::<Config>());
            assert_eq!(child.resolve_binding_id(TypeId::of::<Config>()), Id(1));
        }

        assert_eq!(cx.insert(Database("primary")), None);
        let database_id = cx.resolve_binding_id(TypeId::of::<Database>());

        assert_eq!(config_none_id, Id(0));
        assert_eq!(database_id, Id(2));
        assert_eq!(cx.request_state.bindings.count(), 3);
        assert!(cx.binding_mask.bits.contains(config_none_id.0));
        assert!(cx.binding_mask.bits.contains(database_id.0));
    }

    #[test]
    fn get_mut_changes_a_root_value() {
        let mut cx = CxTestBuilder::new().request_context(Config(1)).build();

        let first_id = cx.resolve_binding_id(TypeId::of::<Config>());
        cx.get_mut::<Config>().unwrap().0 = 42;
        let second_id = cx.resolve_binding_id(TypeId::of::<Config>());

        assert_eq!(request_context::<Config>(&cx), &Config(42));
        assert_ne!(first_id, second_id);
        assert!(!cx.binding_mask.bits.contains(first_id.0));
        assert!(cx.binding_mask.bits.contains(second_id.0));
        let binding_count = cx.request_state.bindings.count();
        let mask = cx.binding_mask.clone();
        assert_eq!(cx.get_mut::<Database>(), None);
        assert_eq!(cx.request_state.bindings.count(), binding_count);
        assert_eq!(cx.binding_mask.bits, mask.bits);
        assert_eq!(cx.binding_mask.frontier, mask.frontier);
    }
}
