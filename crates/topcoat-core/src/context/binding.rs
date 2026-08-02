use std::{any::TypeId, collections::HashMap, sync::Mutex};

use bit_set::BitSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Id(pub(super) usize);

impl Id {
    pub(super) fn frontier(self) -> usize {
        self.0
            .checked_add(1)
            .expect("request context binding ID overflowed")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    RootNone,
    Value,
}

#[derive(Debug, Clone, Default)]
pub(super) struct Mask {
    pub(super) bits: BitSet<usize>,
    pub(super) frontier: usize,
}

impl Mask {
    pub(super) fn install(
        &mut self,
        registry: &Registry,
        type_id: TypeId,
        previous_id: Option<Id>,
    ) -> Id {
        let (root_none_id, binding_id) = registry.allocate_value(type_id);
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
        for index in self.frontier..frontier {
            if registry.kind(Id(index)) == Kind::RootNone {
                self.bits.insert(index);
            }
        }
        self.frontier = frontier;
    }

    pub(super) fn effectively_contains(&self, registry: &Registry, binding_id: Id) -> bool {
        if binding_id.0 < self.frontier {
            self.bits.contains(binding_id.0)
        } else {
            registry.kind(binding_id) == Kind::RootNone
        }
    }
}

#[derive(Debug, Default)]
pub(super) struct Registry {
    root_none: Mutex<HashMap<TypeId, Id>>,
    kinds: boxcar::Vec<Kind>,
}

impl Registry {
    pub(super) fn root_none(&self, type_id: TypeId) -> Id {
        let mut root_none = self.root_none.lock().unwrap();
        *root_none
            .entry(type_id)
            .or_insert_with(|| self.push(Kind::RootNone))
    }

    fn allocate_value(&self, type_id: TypeId) -> (Id, Id) {
        let mut root_none = self.root_none.lock().unwrap();
        let root_none_id = *root_none
            .entry(type_id)
            .or_insert_with(|| self.push(Kind::RootNone));
        (root_none_id, self.push(Kind::Value))
    }

    fn push(&self, kind: Kind) -> Id {
        self.kinds
            .count()
            .checked_add(1)
            .expect("request context binding ID overflowed");
        Id(self.kinds.push(kind))
    }

    fn kind(&self, binding_id: Id) -> Kind {
        *self
            .kinds
            .get(binding_id.0)
            .expect("request context binding metadata was not initialized")
    }

    #[cfg(test)]
    fn count(&self) -> usize {
        self.kinds.count()
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
        assert_eq!(cx.request_state.bindings.kind(first_id), Kind::RootNone);
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
    fn first_value_allocates_and_shadows_root_none() {
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
        let root_none_id = cx
            .request_state
            .bindings
            .root_none(TypeId::of::<Database>());
        let first_id = cx.resolve_binding_id(TypeId::of::<Database>());
        assert!(!cx.binding_mask.bits.contains(root_none_id.0));
        assert!(cx.binding_mask.bits.contains(first_id.0));
        assert_eq!(cx.insert(Database("replica")), Some(Database("primary")));
        let second_id = cx.resolve_binding_id(TypeId::of::<Database>());
        assert_eq!(request_context::<Database>(&cx), &Database("replica"));
        assert_ne!(first_id, second_id);
        assert!(!cx.binding_mask.bits.contains(first_id.0));
        assert!(cx.binding_mask.bits.contains(second_id.0));
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
