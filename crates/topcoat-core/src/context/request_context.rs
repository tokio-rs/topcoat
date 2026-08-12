//! Type-keyed values registered for a single request and dropped when it
//! ends.

use std::{
    any::{Any, TypeId, type_name},
    collections::HashMap,
    fmt,
    hash::{BuildHasherDefault, Hasher},
    sync::Arc,
};

use crate::context::{BindingId, Cx};

/// Returns a reference to the request context value of type `T` registered on
/// the current request's [`Cx`], or `None` if no such value has been registered.
///
/// The lookup is keyed by `T`'s [`TypeId`], so each type may have at most one
/// registered value per request. Request context lives only for the duration
/// of the request that owns it; once the request completes, every value is
/// dropped.
///
/// # Examples
///
/// ```rust
/// use topcoat::context::{Cx, try_request_context};
///
/// struct Customer;
///
/// fn current_customer(cx: &Cx) -> Option<&Customer> {
///     try_request_context(cx)
/// }
/// ```
#[must_use]
pub fn try_request_context<T>(cx: &Cx) -> Option<&T>
where
    T: Any + Send + Sync,
{
    cx.inner.request_context.get::<T>()
}

/// Returns a reference to the request context value of type `T` registered on
/// the current request's [`Cx`].
///
/// The lookup is keyed by `T`'s [`TypeId`], so each type may have at most one
/// registered value per request. Request context lives only for the duration
/// of the request that owns it; once the request completes, every value is
/// dropped.
///
/// # Panics
///
/// Panics if no value of type `T` has been registered on this request's `Cx`.
///
/// # Examples
///
/// ```rust
/// use topcoat::context::{Cx, request_context};
///
/// struct RequestId(String);
///
/// async fn current_request_id(cx: &Cx) -> &str {
///     let id: &RequestId = request_context(cx);
///     &id.0
/// }
/// ```
#[must_use]
#[track_caller]
pub fn request_context<T>(cx: &Cx) -> &T
where
    T: Any + Send + Sync,
{
    match try_request_context(cx) {
        Some(value) => value,
        None => panic!(
            "attempted to access request context of type `{:?}`, but this type was not registered for this context",
            type_name::<T>()
        ),
    }
}

/// The type-keyed values registered for a single request.
///
/// Each value is stored under its [`TypeId`], so a given type can hold one
/// value per request, and is tagged with a [`BindingId`] that is reissued
/// whenever the value is replaced or mutably borrowed, giving every state of a
/// binding a distinct identity. Within a request, values are retrieved with
/// [`request_context`] or [`try_request_context`].
#[derive(Default, Debug)]
pub struct RequestContext {
    entries: HashMap<TypeId, Binding, BuildHasherDefault<TypeIdHasher>>,
}

impl RequestContext {
    /// Creates an empty `RequestContext`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers `value` under its concrete type `T`, returning the value
    /// previously registered for `T`, if any.
    ///
    /// A type can hold only one value at a time, so registering a type that is
    /// already present replaces it and hands back the displaced value. The new
    /// value is registered under a fresh [`BindingId`].
    ///
    /// # Panics
    ///
    /// Panics if the displaced value is still shared and cannot be handed back
    /// by value.
    pub fn insert<T>(&mut self, value: T) -> Option<T>
    where
        T: Any + Send + Sync,
    {
        let binding = Binding {
            id: BindingId::new(),
            value: Arc::new(value),
        };
        let previous = self.entries.insert(TypeId::of::<T>(), binding)?;
        let previous = previous
            .value
            .downcast::<T>()
            .unwrap_or_else(|_| panic!("a request context binding should match its type key"));
        Some(Arc::into_inner(previous).expect("a displaced request context value should be unique"))
    }

    /// Returns a reference to the registered value of type `T`, or `None` if
    /// no such value has been registered.
    ///
    /// Within a request, prefer the [`request_context`] and
    /// [`try_request_context`] free functions over reaching for this directly.
    #[must_use]
    pub fn get<T>(&self) -> Option<&T>
    where
        T: Any + Send + Sync,
    {
        self.entries.get(&TypeId::of::<T>())?.value.downcast_ref()
    }

    /// Returns a mutable reference to the registered value of type `T`, or
    /// `None` if no such value has been registered.
    ///
    /// Borrowing a value mutably reissues its [`BindingId`], since the value
    /// may change under the new borrow.
    #[must_use]
    pub fn get_mut<T>(&mut self) -> Option<&mut T>
    where
        T: Any + Send + Sync,
    {
        let binding = self.entries.get_mut(&TypeId::of::<T>())?;
        binding.id = BindingId::new();
        Arc::get_mut(&mut binding.value)?.downcast_mut()
    }
}

/// One registered value, tagged with the [`BindingId`] issued when it was
/// registered or last mutably borrowed.
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
    use crate::context::CxTestBuilder;

    #[derive(Debug, PartialEq)]
    struct Database(&'static str);

    #[derive(Debug, PartialEq)]
    struct Config(u32);

    /// Returns the id of the binding currently registered for `T`.
    fn binding_id<T: Any>(context: &RequestContext) -> BindingId {
        context.entries[&TypeId::of::<T>()].id
    }

    #[test]
    fn register_and_get_returns_value() {
        let mut context = RequestContext::new();
        context.insert(Database("primary"));

        assert_eq!(context.get::<Database>(), Some(&Database("primary")));
    }

    #[test]
    fn get_returns_none_for_unregistered_type() {
        let context = RequestContext::new();
        assert_eq!(context.get::<Database>(), None);
    }

    #[test]
    fn multiple_types_coexist() {
        let mut context = RequestContext::new();
        context.insert(Database("primary"));
        context.insert(Config(42));

        assert_eq!(context.get::<Database>(), Some(&Database("primary")));
        assert_eq!(context.get::<Config>(), Some(&Config(42)));
    }

    #[test]
    fn insert_replaces_and_returns_the_displaced_value() {
        let mut context = RequestContext::new();
        assert_eq!(context.insert(Database("primary")), None);
        assert_eq!(
            context.insert(Database("replica")),
            Some(Database("primary"))
        );
        assert_eq!(context.get::<Database>(), Some(&Database("replica")));
    }

    #[test]
    fn get_mut_allows_mutation_in_place() {
        let mut context = RequestContext::new();
        context.insert(Config(1));
        context.get_mut::<Config>().unwrap().0 = 42;
        assert_eq!(context.get::<Config>(), Some(&Config(42)));
        assert_eq!(context.get_mut::<Database>(), None);
    }

    #[test]
    fn insert_issues_a_fresh_id() {
        let mut context = RequestContext::new();
        context.insert(Config(1));
        let first = binding_id::<Config>(&context);
        context.insert(Config(2));

        assert_ne!(binding_id::<Config>(&context), first);
    }

    #[test]
    fn get_mut_reissues_the_id() {
        let mut context = RequestContext::new();
        context.insert(Config(1));
        let first = binding_id::<Config>(&context);
        context.get_mut::<Config>().unwrap().0 = 2;

        assert_ne!(binding_id::<Config>(&context), first);
    }

    #[test]
    fn get_keeps_the_id() {
        let mut context = RequestContext::new();
        context.insert(Config(1));
        let first = binding_id::<Config>(&context);
        let _ = context.get::<Config>();

        assert_eq!(binding_id::<Config>(&context), first);
    }

    #[test]
    fn request_context_returns_registered_value() {
        let cx = CxTestBuilder::new()
            .request_context(Database("primary"))
            .build();

        let db: &Database = request_context(&cx);
        assert_eq!(db, &Database("primary"));
    }

    #[test]
    fn try_request_context_returns_registered_value() {
        let cx = CxTestBuilder::new()
            .request_context(Database("primary"))
            .build();

        assert_eq!(
            try_request_context::<Database>(&cx),
            Some(&Database("primary"))
        );
    }

    #[test]
    fn try_request_context_returns_none_for_unregistered_type() {
        let cx = Cx::default();
        assert_eq!(try_request_context::<Database>(&cx), None);
    }

    #[test]
    #[should_panic(expected = "attempted to access request context")]
    fn request_context_panics_for_unregistered_type() {
        let cx = Cx::default();
        let _: &Database = request_context(&cx);
    }
}
