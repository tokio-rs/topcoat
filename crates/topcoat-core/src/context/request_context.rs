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
    if let Some(tracker) = &cx.tracker {
        tracker.record::<T>(&cx.request_context);
    }
    cx.request_context.get::<T>()
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

/// The type-keyed values registered for one scope of a request.
///
/// Each value is stored under its [`TypeId`], so a given type can hold one
/// value per scope, and is tagged with the [`BindingId`] issued when it was
/// registered, giving every binding a distinct identity. Cloning a
/// `RequestContext` shares the values, which is how a child scope inherits
/// them. Within a request, values are retrieved with [`request_context`] or
/// [`try_request_context`].
#[derive(Default, Debug, Clone)]
pub struct RequestContext {
    entries: HashMap<TypeId, Binding, BuildHasherDefault<TypeIdHasher>>,
}

impl RequestContext {
    /// Creates an empty `RequestContext`.
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
/// request context in one step.
///
/// Implemented for tuples of context values, so several types can be
/// registered without deriving a scope per value.
pub trait ContextValues {
    /// Registers every value on `context`.
    fn install(self, context: &mut RequestContext);
}

macro_rules! impl_context_values {
    ($($value:ident: $type:ident),+) => {
        impl<$($type),+> ContextValues for ($($type,)+)
        where
            $($type: Any + Send + Sync),+
        {
            fn install(self, context: &mut RequestContext) {
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
    use crate::context::CxTestBuilder;

    #[derive(Debug, PartialEq)]
    struct Database(&'static str);

    #[derive(Debug, PartialEq)]
    struct Config(u32);

    /// Returns the id of the binding currently registered for `T`.
    fn binding_id<T: Any>(context: &RequestContext) -> BindingId {
        context
            .binding_id(TypeId::of::<T>())
            .expect("binding is registered")
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
    fn insert_replaces_the_previous_value() {
        let mut context = RequestContext::new();
        context.insert(Database("primary"));
        context.insert(Database("replica"));
        assert_eq!(context.get::<Database>(), Some(&Database("replica")));
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
    fn clone_shares_values_and_ids() {
        let mut context = RequestContext::new();
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
        let mut context = RequestContext::new();
        context.insert(Database("primary"));
        let mut clone = context.clone();
        clone.insert(Database("replica"));

        assert_eq!(context.get::<Database>(), Some(&Database("primary")));
        assert_eq!(clone.get::<Database>(), Some(&Database("replica")));
    }

    #[test]
    fn context_values_installs_every_tuple_element() {
        let mut context = RequestContext::new();
        (Database("primary"), Config(42)).install(&mut context);

        assert_eq!(context.get::<Database>(), Some(&Database("primary")));
        assert_eq!(context.get::<Config>(), Some(&Config(42)));
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
