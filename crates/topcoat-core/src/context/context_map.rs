//! Storage for context values, keyed by their Rust type.

use std::{
    any::{Any, TypeId},
    collections::{HashMap, hash_map::Entry},
    fmt,
    hash::{BuildHasherDefault, Hasher},
    iter,
    sync::Arc,
};

use crate::context::BindingId;

/// A map that stores context values by their Rust type.
///
/// Adding a value of a type that is already present shadows the existing
/// value. [`get`](Self::get) returns the newest value of a type, and
/// [`iter`](Self::iter) visits every value of it, newest first. Cloning the
/// map shares its values rather than cloning them. Inserting into a clone
/// does not change the original map.
#[derive(Default, Debug, Clone)]
pub struct ContextMap {
    entries: HashMap<TypeId, Binding, BuildHasherDefault<TypeIdHasher>>,
}

impl ContextMap {
    /// Creates a map with no registered values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Stores `value` under its type `T`.
    ///
    /// If the map already has values of type `T`, `value` shadows them.
    /// [`get`](Self::get) returns `value`, and [`iter`](Self::iter) visits
    /// the older values after it.
    pub fn insert<T>(&mut self, value: T)
    where
        T: Any + Send + Sync,
    {
        self.push(TypeId::of::<T>(), Arc::new(value));
    }

    /// Removes every value of type `T`, including the values it shadows.
    pub fn remove<T>(&mut self)
    where
        T: Any + Send + Sync,
    {
        self.entries.remove(&TypeId::of::<T>());
    }

    /// Adds every value of `values`, removing the values of the same types
    /// from this map first.
    ///
    /// Unlike [`ContextValues::install`], the values of `values` do not
    /// shadow this map's values but replace them. Types that `values` lacks
    /// keep their values.
    pub fn overwrite(&mut self, values: ContextMap) {
        self.entries.extend(values.entries);
    }

    /// Adds `value` under `type_id` with a fresh ID, shadowing the current
    /// binding for that type.
    ///
    /// Every new link gets a fresh ID, so a [`BindingId`] identifies a value
    /// together with all the values it shadows.
    fn push(&mut self, type_id: TypeId, value: Arc<dyn Any + Send + Sync>) {
        let previous = self.entries.remove(&type_id).map(Arc::new);
        let binding = Binding {
            id: BindingId::new(),
            value,
            previous,
        };
        self.entries.insert(type_id, binding);
    }

    /// Reports whether this map contains a value of type `T`.
    #[must_use]
    pub fn contains<T>(&self) -> bool
    where
        T: Any + Send + Sync,
    {
        self.entries.contains_key(&TypeId::of::<T>())
    }

    /// Borrows the value stored under `T`, or returns `None` if it is absent.
    #[must_use]
    pub fn get<T>(&self) -> Option<&T>
    where
        T: Any + Send + Sync,
    {
        Some(self.lookup::<T>()?.1)
    }

    /// Finds `T` and returns its binding ID and a reference to its value.
    /// Returns `None` when the map has no entry for `T`.
    ///
    /// Tracking needs the ID to remember which value was read, while the
    /// caller needs the value. Find both in a single lookup.
    pub(crate) fn lookup<T>(&self) -> Option<(BindingId, &T)>
    where
        T: Any + Send + Sync,
    {
        let binding = self.entries.get(&TypeId::of::<T>())?;
        Some((binding.id, binding.value.downcast_ref()?))
    }

    /// Visits every value of type `T`, from newest to oldest.
    ///
    /// The first item is the value that [`get`](Self::get) returns. The
    /// iterator is empty if the map has no value of type `T`.
    pub fn iter<T>(&self) -> impl Iterator<Item = &T>
    where
        T: Any + Send + Sync,
    {
        iter::successors(self.entries.get(&TypeId::of::<T>()), |binding| {
            binding.previous.as_deref()
        })
        .filter_map(|binding| binding.value.downcast_ref())
    }

    /// Returns the binding ID for `type_id`, or `None` if it is absent.
    ///
    /// A [`TypeId`] lets tracking check a saved read without a type parameter.
    pub(crate) fn binding_id(&self, type_id: TypeId) -> Option<BindingId> {
        Some(self.entries.get(&type_id)?.id)
    }
}

/// A group of values that can be added to a [`ContextMap`] together.
///
/// Pass a tuple or a [`ContextMap`] to
/// [`Cx::with_many`](crate::context::Cx::with_many) to add several values
/// in one call.
pub trait ContextValues {
    /// Adds all of these values to `context`.
    fn install(self, context: &mut ContextMap);
}

/// Adds the values of each type from oldest to newest, so they shadow the
/// existing values of that type in the same order as in this map.
impl ContextValues for ContextMap {
    fn install(self, context: &mut ContextMap) {
        for (type_id, binding) in self.entries {
            if let Entry::Vacant(entry) = context.entries.entry(type_id) {
                // Nothing to shadow, so the binding and its IDs stay valid.
                entry.insert(binding);
                continue;
            }
            let values: Vec<_> = iter::successors(Some(&binding), |binding| {
                binding.previous.as_deref()
            })
            .map(|binding| Arc::clone(&binding.value))
            .collect();
            for value in values.into_iter().rev() {
                context.push(type_id, value);
            }
        }
    }
}

macro_rules! impl_context_values {
    ($($value:ident: $type:ident),+) => {
        impl<$($type),+> ContextValues for ($($type,)+)
        where
            $($type: Any + Send + Sync),+
        {
            fn install(self, context: &mut ContextMap) {
                let ($($value,)+) = self;
                $(context.insert($value);)+
            }
        }
    };
}

impl_context_values!(a: A);
impl_context_values!(a: A, b: B);
impl_context_values!(a: A, b: B, c: C);
impl_context_values!(a: A, b: B, c: C, d: D);
impl_context_values!(a: A, b: B, c: C, d: D, e: E);
impl_context_values!(a: A, b: B, c: C, d: D, e: E, f: F);
impl_context_values!(a: A, b: B, c: C, d: D, e: E, f: F, g: G);
impl_context_values!(a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H);

/// A stored value and the [`BindingId`] assigned when it was added.
#[derive(Clone)]
struct Binding {
    id: BindingId,
    value: Arc<dyn Any + Send + Sync>,
    /// The value of the same type that this one shadows, if any.
    previous: Option<Arc<Binding>>,
}

impl fmt::Debug for Binding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Binding")
            .field("id", &self.id)
            .field("previous", &self.previous)
            .finish_non_exhaustive()
    }
}

/// Passes a [`TypeId`]'s own hash bits through as the map hash.
///
/// A `TypeId` is already a high quality hash of the type it names, so running
/// it through a general purpose hasher only adds work. This hasher keeps the
/// bits it is fed instead of mixing them.
#[derive(Default)]
struct TypeIdHasher(u64);

impl Hasher for TypeIdHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 = self.0.rotate_left(8) ^ u64::from(byte);
        }
    }

    fn write_u64(&mut self, n: u64) {
        self.0 = n;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct Database(&'static str);

    #[derive(Debug, PartialEq)]
    struct Config(u32);

    /// Returns the id of the binding currently registered for `T`.
    fn binding_id<T: Any>(context: &ContextMap) -> BindingId {
        context
            .binding_id(TypeId::of::<T>())
            .expect("binding is registered")
    }

    #[test]
    fn register_and_get_returns_value() {
        let mut context = ContextMap::new();
        context.insert(Database("primary"));

        assert_eq!(context.get::<Database>(), Some(&Database("primary")));
    }

    #[test]
    fn get_returns_none_for_unregistered_type() {
        let context = ContextMap::new();
        assert_eq!(context.get::<Database>(), None);
    }

    #[test]
    fn multiple_types_coexist() {
        let mut context = ContextMap::new();
        context.insert(Database("primary"));
        context.insert(Config(42));

        assert_eq!(context.get::<Database>(), Some(&Database("primary")));
        assert_eq!(context.get::<Config>(), Some(&Config(42)));
    }

    #[test]
    fn insert_shadows_the_previous_value() {
        let mut context = ContextMap::new();
        context.insert(Config(1));
        context.insert(Config(2));
        context.insert(Config(3));

        assert_eq!(context.get::<Config>(), Some(&Config(3)));
        assert_eq!(
            context.iter::<Config>().collect::<Vec<_>>(),
            [&Config(3), &Config(2), &Config(1)]
        );
    }

    #[test]
    fn insert_issues_a_fresh_id() {
        let mut context = ContextMap::new();
        context.insert(Config(1));
        let first = binding_id::<Config>(&context);
        context.insert(Config(2));

        assert_ne!(binding_id::<Config>(&context), first);
    }

    #[test]
    fn iter_is_empty_for_unregistered_type() {
        let mut context = ContextMap::new();
        context.insert(Database("primary"));

        assert_eq!(context.iter::<Config>().count(), 0);
    }

    #[test]
    fn remove_drops_every_value_of_the_type() {
        let mut context = ContextMap::new();
        context.insert(Config(1));
        context.insert(Config(2));
        context.insert(Database("primary"));
        context.remove::<Config>();

        assert_eq!(context.iter::<Config>().count(), 0);
        assert_eq!(context.get::<Database>(), Some(&Database("primary")));
    }

    #[test]
    fn overwrite_replaces_values_of_the_same_types() {
        let mut context = ContextMap::new();
        context.insert(Config(1));
        context.insert(Config(2));
        context.insert(Database("primary"));
        let mut values = ContextMap::new();
        values.insert(Config(3));
        context.overwrite(values);

        assert_eq!(context.iter::<Config>().collect::<Vec<_>>(), [&Config(3)]);
        assert_eq!(context.get::<Database>(), Some(&Database("primary")));
    }

    #[test]
    fn contains_reports_registered_types() {
        let mut context = ContextMap::new();
        assert!(!context.contains::<Database>());
        context.insert(Database("primary"));
        assert!(context.contains::<Database>());
        assert!(!context.contains::<Config>());
    }

    #[test]
    fn clone_shares_values_and_ids() {
        let mut context = ContextMap::new();
        context.insert(Database("primary"));
        let clone = context.clone();

        assert_eq!(clone.get::<Database>(), Some(&Database("primary")));
        assert_eq!(
            binding_id::<Database>(&clone),
            binding_id::<Database>(&context)
        );
    }

    #[test]
    fn inserting_into_a_clone_leaves_the_original_untouched() {
        let mut context = ContextMap::new();
        context.insert(Database("primary"));
        let mut clone = context.clone();
        clone.insert(Database("replica"));

        assert_eq!(
            context.iter::<Database>().collect::<Vec<_>>(),
            [&Database("primary")]
        );
        assert_eq!(
            clone.iter::<Database>().collect::<Vec<_>>(),
            [&Database("replica"), &Database("primary")]
        );
    }

    #[test]
    fn context_values_installs_every_tuple_element() {
        let mut context = ContextMap::new();
        (Database("primary"), Config(42)).install(&mut context);

        assert_eq!(context.get::<Database>(), Some(&Database("primary")));
        assert_eq!(context.get::<Config>(), Some(&Config(42)));
    }

    #[test]
    fn installing_a_tuple_shadows_in_order() {
        let mut context = ContextMap::new();
        context.insert(Config(1));
        (Config(2), Config(3)).install(&mut context);

        assert_eq!(
            context.iter::<Config>().collect::<Vec<_>>(),
            [&Config(3), &Config(2), &Config(1)]
        );
    }

    #[test]
    fn installing_a_map_shadows_in_order() {
        let mut context = ContextMap::new();
        context.insert(Config(0));
        let mut values = ContextMap::new();
        values.insert(Config(1));
        values.insert(Config(2));
        let installed = binding_id::<Config>(&values);
        values.install(&mut context);

        assert_eq!(
            context.iter::<Config>().collect::<Vec<_>>(),
            [&Config(2), &Config(1), &Config(0)]
        );
        assert_ne!(binding_id::<Config>(&context), installed);
    }

    #[test]
    fn installing_a_map_without_overlap_keeps_its_ids() {
        let mut context = ContextMap::new();
        context.insert(Database("primary"));
        let mut values = ContextMap::new();
        values.insert(Config(1));
        values.insert(Config(2));
        let installed = binding_id::<Config>(&values);
        values.install(&mut context);

        assert_eq!(
            context.iter::<Config>().collect::<Vec<_>>(),
            [&Config(2), &Config(1)]
        );
        assert_eq!(binding_id::<Config>(&context), installed);
    }
}
