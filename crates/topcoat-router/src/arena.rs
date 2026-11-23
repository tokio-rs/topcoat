use bumpalo::Bump;
use self_cell::{MutBorrow, self_cell};

use crate::{Layer, Layout, Page, Route};

/// An arena slot that owns a value and can drop it in place.
///
/// A bump arena frees its memory without running destructors, so every value
/// that needs dropping is stored as a slot. `D` is the type the value is read
/// as, such as `dyn Layer`.
pub(crate) trait ArenaSlot<D: ?Sized>: Send + Sync {
    /// Borrows the value.
    fn get(&self) -> &D;

    /// Drops the value. The slot must not be read afterwards.
    fn clear(&mut self);
}

impl<T: Route> ArenaSlot<dyn Route> for Option<T> {
    fn get(&self) -> &dyn Route {
        self.as_ref()
            .expect("an arena slot is read before it is cleared")
    }

    fn clear(&mut self) {
        *self = None;
    }
}

impl<T: Page> ArenaSlot<dyn Page> for Option<T> {
    fn get(&self) -> &dyn Page {
        self.as_ref()
            .expect("an arena slot is read before it is cleared")
    }

    fn clear(&mut self) {
        *self = None;
    }
}

impl<T: Layout> ArenaSlot<dyn Layout> for Option<T> {
    fn get(&self) -> &dyn Layout {
        self.as_ref()
            .expect("an arena slot is read before it is cleared")
    }

    fn clear(&mut self) {
        *self = None;
    }
}

impl<T: Layer> ArenaSlot<dyn Layer> for Option<T> {
    fn get(&self) -> &dyn Layer {
        self.as_ref()
            .expect("an arena slot is read before it is cleared")
    }

    fn clear(&mut self) {
        *self = None;
    }
}

/// The values of one kind stored in an arena, in the order they were added.
///
/// This list owns the values. Dropping it drops each of them in place.
pub(crate) struct ArenaSlots<'r, D: ?Sized + 'static> {
    slots: Vec<&'r mut (dyn ArenaSlot<D> + 'static)>,
}

impl<'r, D: ?Sized + 'static> ArenaSlots<'r, D> {
    /// Moves `value` into `arena` and takes ownership of it.
    pub(crate) fn push<T>(&mut self, arena: &'r Bump, value: T)
    where
        T: 'static,
        Option<T>: ArenaSlot<D>,
    {
        self.slots.push(arena.alloc(Some(value)));
    }

    /// Iterates over the values in the order they were added.
    pub(crate) fn iter(&self) -> impl ExactSizeIterator<Item = &D> {
        self.slots.iter().map(|slot| slot.get())
    }
}

impl<D: ?Sized + 'static> Default for ArenaSlots<'_, D> {
    fn default() -> Self {
        Self { slots: Vec::new() }
    }
}

impl<D: ?Sized + 'static> Drop for ArenaSlots<'_, D> {
    fn drop(&mut self) {
        for slot in &mut self.slots {
            slot.clear();
        }
    }
}

/// Every value in the arena that needs dropping, grouped by kind.
#[derive(Default)]
pub(crate) struct Objects<'r> {
    pub(crate) routes: ArenaSlots<'r, dyn Route>,
    pub(crate) pages: ArenaSlots<'r, dyn Page>,
    pub(crate) layouts: ArenaSlots<'r, dyn Layout>,
    pub(crate) layers: ArenaSlots<'r, dyn Layer>,
}

self_cell!(
    /// The arena of a router that is still being built, together with the
    /// values registered in it so far.
    pub(crate) struct Staged {
        owner: Bump,
        #[covariant]
        dependent: Objects,
    }
);

impl Default for Staged {
    fn default() -> Self {
        Self::new(Bump::new(), |_| Objects::default())
    }
}

/// What a built router reads to serve requests.
///
/// The references point into the arena of the [`RouterCell`] that owns these
/// tables.
pub(crate) struct Tables<'r> {
    /// The layers that run around requests matching no route.
    pub(crate) always_layers: &'r [&'r dyn Layer],
}

self_cell!(
    /// A built router: the arena with its registered values and the
    /// [`Tables`] that point into it.
    pub(crate) struct RouterCell {
        owner: MutBorrow<Staged>,
        #[not_covariant]
        dependent: Tables,
    }
);

impl RouterCell {
    /// Finishes `staged` into a router.
    ///
    /// `tables` receives the arena and the registered values. It can allocate
    /// in the arena, and the tables it returns can point at anything in it.
    pub(crate) fn build(
        staged: Staged,
        tables: impl for<'r> FnOnce(&'r Bump, &'r Objects<'r>) -> Tables<'r>,
    ) -> Self {
        Self::new(MutBorrow::new(staged), |staged| {
            let staged: &Staged = staged.borrow_mut();
            tables(staged.borrow_owner(), staged.borrow_dependent())
        })
    }

    /// Returns the layers that run around requests matching no route.
    pub(crate) fn always_layers(&self) -> &[&dyn Layer] {
        self.with_dependent(|_, tables| tables.always_layers)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        thread,
    };

    use topcoat_core::context::Cx;
    use topcoat_view::BoxView;

    use super::*;
    use crate::{
        Body, LayerFn, LayerFuture, LayoutFn, Method, Next, PageFn, Path, RouteFn, RouteFuture,
        Slot,
    };

    /// Counts how many guards were dropped.
    #[derive(Default)]
    struct Drops(Arc<AtomicUsize>);

    impl Drops {
        fn guard(&self) -> Guard {
            Guard(Arc::clone(&self.0))
        }

        fn count(&self) -> usize {
            self.0.load(Ordering::Relaxed)
        }
    }

    /// A value with a destructor, owned by a registered route or layer.
    struct Guard(Arc<AtomicUsize>);

    impl Drop for Guard {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn unused_route(_cx: &Cx, _body: Body) -> RouteFuture<'_> {
        unreachable!()
    }

    fn unused_page(_cx: &Cx, _body: Body) -> BoxView<'_> {
        unreachable!()
    }

    fn unused_layout<'a>(_cx: &'a Cx, _slot: Slot<'a>) -> BoxView<'a> {
        unreachable!()
    }

    fn unused_layer<'a>(_cx: &'a Cx, _body: Body, _next: Next<'a>) -> LayerFuture<'a> {
        unreachable!()
    }

    /// An arena holding one value of each kind. Each value owns a guard.
    fn staged_with_guards(drops: &Drops) -> Staged {
        let mut staged = Staged::default();
        staged.with_dependent_mut(|arena, objects| {
            let route = RouteFn::new(Method::GET, "/route", unused_route);
            objects.routes.push(arena, route.with(drops.guard()));
            let page = PageFn::new(Method::GET, "/page", unused_page);
            objects.pages.push(arena, page.with(drops.guard()));
            let layout = LayoutFn::new("/layout", unused_layout);
            objects.layouts.push(arena, layout.with(drops.guard()));
            let layer = LayerFn::new(Some("/layer"), unused_layer);
            objects.layers.push(arena, layer.with(drops.guard()));
        });
        staged
    }

    fn build_with_all_layers(staged: Staged) -> RouterCell {
        RouterCell::build(staged, |arena, objects| Tables {
            always_layers: arena.alloc_slice_fill_iter(objects.layers.iter()),
        })
    }

    #[test]
    fn an_unbuilt_arena_drops_its_values() {
        let drops = Drops::default();
        let staged = staged_with_guards(&drops);
        assert_eq!(drops.count(), 0);

        drop(staged);
        assert_eq!(drops.count(), 4);
    }

    #[test]
    fn a_router_drops_its_values_with_its_last_handle() {
        let drops = Drops::default();
        let router = Arc::new(build_with_all_layers(staged_with_guards(&drops)));
        let other = Arc::clone(&router);

        drop(router);
        assert_eq!(drops.count(), 0);

        // The last handle can be read and dropped on any thread.
        let layers = thread::spawn(move || other.always_layers().len())
            .join()
            .unwrap();
        assert_eq!(layers, 1);
        assert_eq!(drops.count(), 4);
    }

    #[test]
    fn the_tables_read_values_in_registration_order() {
        let mut staged = Staged::default();
        staged.with_dependent_mut(|arena, objects| {
            for path in ["/first", "/second", "/third"] {
                objects
                    .layers
                    .push(arena, LayerFn::new(Some(path), unused_layer));
            }
        });
        let router = build_with_all_layers(staged);

        let paths: Vec<&str> = router
            .always_layers()
            .iter()
            .filter_map(|&layer| layer.path())
            .map(Path::as_str)
            .collect();
        assert_eq!(paths, ["/first", "/second", "/third"]);
    }

    #[test]
    fn the_arena_moves_between_threads_and_the_router_is_shared() {
        fn assert_send<T: Send>() {}
        fn assert_send_sync<T: Send + Sync>() {}

        assert_send::<Staged>();
        assert_send_sync::<RouterCell>();
    }
}
