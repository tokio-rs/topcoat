//! Storage for context values, with one entry per Rust type.

use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fmt,
    hash::{BuildHasherDefault, Hasher},
    sync::Arc,
};

use crate::context::BindingId;

/// A map that stores one value for each Rust type.
///
/// Adding another value of the same type replaces the existing one.
/// Cloning the map shares its values rather than cloning them. Inserting
/// into a clone does not change the original map.
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
    /// Replaces any value already stored under that type.
    pub fn insert<T>(&mut self, value: T)
    where
        T: Any + Send + Sync,
    {
        let binding = Binding {
            id: BindingId::new(),
            value: Arc::new(value),
        };
        self.entries.insert(TypeId::of::<T>(), binding);
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

impl ContextValues for ContextMap {
    fn install(self, context: &mut ContextMap) {
        context.entries.extend(self.entries);
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
}

impl fmt::Debug for Binding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Binding")
            .field("id", &self.id)
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
    fn insert_replaces_the_previous_value() {
        let mut context = ContextMap::new();
        context.insert(Database("primary"));
        context.insert(Database("replica"));
        assert_eq!(context.get::<Database>(), Some(&Database("replica")));
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

        assert_eq!(context.get::<Database>(), Some(&Database("primary")));
        assert_eq!(clone.get::<Database>(), Some(&Database("replica")));
    }

    #[test]
    fn context_values_installs_every_tuple_element() {
        let mut context = ContextMap::new();
        (Database("primary"), Config(42)).install(&mut context);

        assert_eq!(context.get::<Database>(), Some(&Database("primary")));
        assert_eq!(context.get::<Config>(), Some(&Config(42)));
    }
}
