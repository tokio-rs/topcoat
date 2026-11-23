use std::sync::Arc;

use bumpalo::Bump;
use self_cell::{MutBorrow, self_cell};
use topcoat_core::context::ContextMap;

use crate::{Endpoint, Layer, Layout, Page, RegisteredRoute, Route, RouteId, TrustedProxies};

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

    /// Returns `true` if no value was added.
    pub(crate) fn is_empty(&self) -> bool {
        self.slots.is_empty()
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
    /// Resolves a request path to its endpoint.
    pub(crate) matcher: matchit::Router<Endpoint<'r>>,
    /// The layers that run around requests matching no route.
    pub(crate) always_layers: &'r [&'r dyn Layer],
    /// Every route the application registered, sorted by id.
    pub(crate) declared: &'r [RegisteredRoute<'r>],
    /// `declared` indexed by id, or `None` when the ids are too sparse for a table.
    pub(crate) by_id: Option<&'r [Option<&'r RegisteredRoute<'r>>]>,
    /// The values registered on the router builder.
    pub(crate) context: Arc<ContextMap>,
    /// The proxies trusted to report the client's address.
    pub(crate) trusted_proxies: TrustedProxies,
}

impl<'r> Tables<'r> {
    /// Returns `declared` as a table with one slot per id, up to the highest
    /// id in it. The slot of an id that no declared route has is empty.
    pub(crate) fn index_by_id(
        arena: &'r Bump,
        declared: &'r [RegisteredRoute<'r>],
    ) -> &'r [Option<&'r RegisteredRoute<'r>>] {
        let len = declared
            .iter()
            .map(|route| route.id().number() + 1)
            .max()
            .unwrap_or(0);
        let slots = arena.alloc_slice_fill_copy(len, None);
        for route in declared {
            slots[route.id().number()] = Some(route);
        }
        slots
    }
}

/// Everything the [`Tables`] of a built router point at.
pub(crate) struct Owner {
    staged: Staged,
    contexts: Vec<Arc<ContextMap>>,
}

self_cell!(
    /// A built router: the arena with its registered values and the
    /// [`Tables`] that point into it.
    pub(crate) struct RouterCell {
        owner: MutBorrow<Owner>,
        #[not_covariant]
        dependent: Tables,
    }
);

impl RouterCell {
    /// Finishes `staged` into a router.
    ///
    /// `tables` receives the arena, the registered values and an empty list
    /// of contexts. It can allocate in the arena and push to the list, and
    /// the tables it returns can point at anything in either.
    pub(crate) fn build(
        staged: Staged,
        tables: impl for<'r> FnOnce(
            &'r Bump,
            &'r Objects<'r>,
            &'r mut Vec<Arc<ContextMap>>,
        ) -> Tables<'r>,
    ) -> Self {
        let owner = Owner {
            staged,
            contexts: Vec::new(),
        };
        Self::new(MutBorrow::new(owner), |owner| {
            let Owner { staged, contexts } = owner.borrow_mut();
            let staged: &Staged = staged;
            tables(staged.borrow_owner(), staged.borrow_dependent(), contexts)
        })
    }

    /// Matches a request path to its endpoint and the path parameters it
    /// captured, or returns `None` if no endpoint serves the path.
    pub(crate) fn at<'s, 'p>(
        &'s self,
        path: &'p str,
    ) -> Option<(&'s Endpoint<'s>, matchit::Params<'s, 'p>)> {
        self.with_dependent(|_, tables| {
            let matched = tables.matcher.at(path).ok()?;
            Some((matched.value, matched.params))
        })
    }

    /// Returns the layers that run around requests matching no route.
    pub(crate) fn always_layers(&self) -> &[&dyn Layer] {
        self.with_dependent(|_, tables| tables.always_layers)
    }

    /// Returns every route the application registered, sorted by id.
    pub(crate) fn declared(&self) -> &[RegisteredRoute<'_>] {
        self.with_dependent(|_, tables| tables.declared)
    }

    /// Returns the route registered under `id`, or `None` if the application
    /// registered no route with that identity.
    pub(crate) fn route(&self, id: RouteId) -> Option<RegisteredRoute<'_>> {
        if let Some(by_id) = self.by_id() {
            return by_id.get(id.number()).copied().flatten().copied();
        }
        let declared = self.declared();
        let position = declared
            .binary_search_by_key(&id.number(), |route| route.id().number())
            .ok()?;
        Some(declared[position])
    }

    fn by_id(&self) -> Option<&[Option<&RegisteredRoute<'_>>]> {
        self.with_dependent(|_, tables| tables.by_id)
    }

    /// Returns the values registered on the router builder.
    pub(crate) fn context(&self) -> &Arc<ContextMap> {
        self.with_dependent(|_, tables| &tables.context)
    }

    /// Returns the proxies trusted to report the client's address.
    pub(crate) fn trusted_proxies(&self) -> &TrustedProxies {
        self.with_dependent(|_, tables| &tables.trusted_proxies)
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
        Body, Handler, LayerFn, LayerFuture, LayoutFn, Method, Next, PageFn, Path, RouteEntry,
        RouteFn, RouteFuture, Slot,
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

    /// Tables with no endpoints and no routes.
    fn empty_tables<'r>() -> Tables<'r> {
        Tables {
            matcher: matchit::Router::new(),
            always_layers: &[],
            declared: &[],
            by_id: None,
            context: Arc::new(ContextMap::new()),
            trusted_proxies: TrustedProxies::new(),
        }
    }

    fn build_with_all_layers(staged: Staged) -> RouterCell {
        RouterCell::build(staged, |arena, objects, _| Tables {
            always_layers: arena.alloc_slice_fill_iter(objects.layers.iter()),
            ..empty_tables()
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
    fn a_path_matches_its_endpoint_and_captures_its_params() {
        let router = RouterCell::build(Staged::default(), |arena, _, _| {
            let path = Path::new(arena.alloc_str("/users/{id}"));
            let mut matcher = matchit::Router::new();
            matcher.insert(path.as_str(), Endpoint::new(path)).unwrap();
            Tables {
                matcher,
                ..empty_tables()
            }
        });

        let (endpoint, params) = router.at("/users/42").unwrap();
        assert_eq!(endpoint.path(), Path::new("/users/{id}"));
        assert_eq!(params.get("id"), Some("42"));
        assert!(router.at("/missing").is_none());
    }

    #[test]
    fn a_route_is_found_by_its_id_with_and_without_the_id_table() {
        for indexed in [true, false] {
            let first = RouteFn::new(Method::GET, "/first", unused_route);
            let second = RouteFn::new(Method::GET, "/second", unused_route);
            // An id between those of the routes that belongs to none of them.
            let unregistered = RouteId::new();
            let third = RouteFn::new(Method::GET, "/third", unused_route);
            let ids = [first.id(), second.id(), third.id()];

            let mut staged = Staged::default();
            staged.with_dependent_mut(|arena, objects| {
                for route in [first, second, third] {
                    objects.routes.push(arena, route);
                }
            });
            let router = RouterCell::build(staged, |arena, objects, _| {
                // Ids are drawn in order, so the routes are sorted by id.
                let declared: Vec<RegisteredRoute<'_>> = objects
                    .routes
                    .iter()
                    .map(|route| {
                        let entry = arena.alloc(RouteEntry {
                            handler: Handler::Route(route),
                            layers: &[],
                            context: None,
                        });
                        RegisteredRoute::new(route.path(), entry)
                    })
                    .collect();
                let declared = arena.alloc_slice_copy(&declared);
                Tables {
                    declared,
                    by_id: indexed.then(|| Tables::index_by_id(arena, declared)),
                    ..empty_tables()
                }
            });

            let paths: Vec<Option<&str>> = ids
                .iter()
                .map(|&id| router.route(id).map(|route| route.path().as_str()))
                .collect();
            assert_eq!(
                paths,
                [Some("/first"), Some("/second"), Some("/third")],
                "indexed: {indexed}"
            );
            assert!(router.route(unregistered).is_none(), "indexed: {indexed}");
            // An id drawn after every route is past the end of the table.
            assert!(router.route(RouteId::new()).is_none(), "indexed: {indexed}");
        }
    }

    #[test]
    fn the_arena_moves_between_threads_and_the_router_is_shared() {
        fn assert_send<T: Send>() {}
        fn assert_send_sync<T: Send + Sync>() {}

        assert_send::<Staged>();
        assert_send_sync::<RouterCell>();
    }
}
