//! The type-keyed map behind every layer of context.

use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fmt,
    hash::{BuildHasherDefault, Hasher},
    sync::Arc,
};

use crate::context::BindingId;

/// Values identified by their type.
///
/// Stores one value per Rust type. Each value is registered with a fresh
/// [`BindingId`], which context tracking uses to tell values apart. Clones
/// share the registered values, so cloning a map is cheap and inserting into
/// a clone leaves the original untouched.
#[derive(Default, Debug, Clone)]
pub struct ContextMap {
    entries: HashMap<TypeId, Binding, BuildHasherDefault<TypeIdHasher>>,
}

impl ContextMap {
    /// Creates an empty `ContextMap`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers `value` under its concrete type `T` with a fresh
    /// [`BindingId`].
    ///
    /// A type can hold only one value at a time, so registering a type that is
    /// already present replaces the previous value.
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

    /// Returns `true` if a value of type `T` has been registered.
    #[must_use]
    pub fn contains<T>(&self) -> bool
    where
        T: Any + Send + Sync,
    {
        self.entries.contains_key(&TypeId::of::<T>())
    }

    /// Returns a reference to the registered value of type `T`, or `None` if
    /// no such value has been registered.
    #[must_use]
    pub fn get<T>(&self) -> Option<&T>
    where
        T: Any + Send + Sync,
    {
        Some(self.lookup::<T>()?.1)
    }

    /// Returns a mutable reference to the registered value of type `T`, or
    /// `None` if no such value has been registered.
    ///
    /// Also returns `None` while a clone of this map shares the value, since
    /// the value cannot be changed without changing it for the clone too.
    #[must_use]
    pub fn get_mut<T>(&mut self) -> Option<&mut T>
    where
        T: Any + Send + Sync,
    {
        let binding = self.entries.get_mut(&TypeId::of::<T>())?;
        Arc::get_mut(&mut binding.value)?.downcast_mut()
    }

    /// Returns the registered value of type `T` together with the id of the
    /// binding holding it, or `None` if no such value has been registered.
    ///
    /// A tracked read needs both: the id to record what it observed, and the
    /// value to hand back. Resolving them together keeps that to one lookup.
    pub(crate) fn lookup<T>(&self) -> Option<(BindingId, &T)>
    where
        T: Any + Send + Sync,
    {
        let binding = self.entries.get(&TypeId::of::<T>())?;
        Some((binding.id, binding.value.downcast_ref()?))
    }

    /// Returns the id of the binding currently registered for `type_id`, or
    /// `None` if the type is not registered.
    ///
    /// Unlike [`get`](Self::get), this works from a runtime [`TypeId`], so a
    /// recorded read can be revalidated without naming its type.
    pub(crate) fn binding_id(&self, type_id: TypeId) -> Option<BindingId> {
        Some(self.entries.get(&type_id)?.id)
    }
}

/// Values that [`Cx::with_many`](crate::context::Cx::with_many) registers on a
/// [`ContextMap`] in one step.
///
/// Implemented for tuples of context values and [`ContextMap`], so several
/// types can be registered without deriving a scope per value.
pub trait ContextValues {
    /// Registers every value on `context`.
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

/// One registered value, tagged with the [`BindingId`] issued when it was
/// registered.
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
    fn get_mut_allows_mutation_in_place() {
        let mut context = ContextMap::new();
        context.insert(Config(1));
        let id = binding_id::<Config>(&context);
        context.get_mut::<Config>().unwrap().0 = 42;

        assert_eq!(context.get::<Config>(), Some(&Config(42)));
        assert_eq!(binding_id::<Config>(&context), id);
        assert_eq!(context.get_mut::<Database>(), None);
    }

    #[test]
    fn get_mut_returns_none_while_a_clone_shares_the_value() {
        let mut context = ContextMap::new();
        context.insert(Config(1));
        let clone = context.clone();

        assert_eq!(context.get_mut::<Config>(), None);
        drop(clone);
        assert_eq!(context.get_mut::<Config>(), Some(&mut Config(1)));
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

    #[test]
    fn get_keeps_the_id() {
        let mut context = ContextMap::new();
        context.insert(Config(1));
        let first = binding_id::<Config>(&context);
        let _ = context.get::<Config>();

        assert_eq!(binding_id::<Config>(&context), first);
    }
}
