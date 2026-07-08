//! Type-keyed values made available through the request context.
//!
//! [`ContextMap`] is a type-keyed map of values, looked up by their [`TypeId`](std::any::TypeId).
//! Each [`Cx`] carries two of them:
//!
//! - **App context** is registered once at startup and shared across every request handled by the
//!   router. Within a request, [`app_context`] retrieves a reference to a registered value by its
//!   type.
//! - **Request context** is scoped to a single request and dropped when the request ends. Within a
//!   request, [`request_context`] retrieves a reference to a registered value by its type.

use std::any::{Any, type_name};

use crate::runtime::context::Cx;

/// Returns a reference to the app context value of type `T` registered on the
/// router.
///
/// The lookup is keyed by `T`'s [`TypeId`](std::any::TypeId), so each type may have at most one
/// registered value.
///
/// # Panics
///
/// Panics if no value of type `T` has been registered.
///
/// # Examples
///
/// ```rust
/// # struct User;
/// # impl Database {
/// #     async fn fetch_user(&self, id: u64) -> User { User }
/// # }
/// use topcoat::context::{Cx, app_context};
///
/// struct Database {/* ... */}
///
/// async fn load_user(cx: &Cx, id: u64) -> User {
///     let db: &Database = app_context(cx);
///     db.fetch_user(id).await
/// }
/// ```
pub fn app_context<T>(cx: &Cx) -> &T
where
    T: Any + Send + Sync,
{
    match cx.app_context.get::<T>() {
        Some(value) => value,
        None => panic!(
            "attempted to access app context of type `{:?}`, but this type was not registered for this context",
            type_name::<T>()
        ),
    }
}

/// Returns a reference to the request context value of type `T` registered on
/// the current request's [`Cx`].
///
/// The lookup is keyed by `T`'s [`TypeId`](std::any::TypeId), so each type may have at most one
/// registered value per request. Request context lives only for the duration of
/// the request that owns it; once the request completes, every value is
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
pub fn request_context<T>(cx: &Cx) -> &T
where
    T: Any + Send + Sync,
{
    match cx.request_context.get::<T>() {
        Some(value) => value,
        None => panic!(
            "attempted to access request context of type `{:?}`, but this type was not registered for this context",
            type_name::<T>()
        ),
    }
}

/// A type-keyed container of values.
///
/// Each registered value is stored under its [`TypeId`](std::any::TypeId), so a given type can
/// only be registered once per `ContextMap`. Used by [`Cx`] to hold both the
/// router-wide app context and the per-request request context; values are
/// retrieved within a request via [`app_context`] or [`request_context`].
#[derive(Default, Debug)]
pub struct ContextMap {
    entries: anymap3::Map<dyn Any + Send + Sync>,
}

impl ContextMap {
    /// Creates an empty `ContextMap`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers `value` under its concrete type `T`, returning the value
    /// previously registered for `T`, if any.
    ///
    /// A type can hold only one value at a time, so registering a type that is
    /// already present replaces it and hands back the displaced value.
    pub fn insert<T>(&mut self, value: T) -> Option<T>
    where
        T: Any + Send + Sync,
    {
        self.entries.insert::<T>(value)
    }

    /// Returns `true` if a value of type `T` has been registered.
    #[must_use]
    pub fn contains<T>(&self) -> bool
    where
        T: Any + Send + Sync,
    {
        self.entries.contains::<T>()
    }

    /// Returns a reference to the registered value of type `T`, or `None` if
    /// no such value has been registered.
    ///
    /// Within a request, prefer the [`app_context`] and [`request_context`] free
    /// functions over reaching for this directly.
    #[must_use]
    pub fn get<T>(&self) -> Option<&T>
    where
        T: Any + Send + Sync,
    {
        self.entries.get::<T>()
    }

    /// Returns a mutable reference to the registered value of type `T`, or
    /// `None` if no such value has been registered.
    #[must_use]
    pub fn get_mut<T>(&mut self) -> Option<&mut T>
    where
        T: Any + Send + Sync,
    {
        self.entries.get_mut::<T>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::context::CxTestBuilder;

    #[derive(Debug, PartialEq)]
    struct Database(&'static str);

    #[derive(Debug, PartialEq)]
    struct Config(u32);

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
    fn insert_replaces_and_returns_the_displaced_value() {
        let mut context = ContextMap::new();
        assert_eq!(context.insert(Database("primary")), None);
        assert_eq!(
            context.insert(Database("replica")),
            Some(Database("primary"))
        );
        assert_eq!(context.get::<Database>(), Some(&Database("replica")));
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
        context.get_mut::<Config>().unwrap().0 = 42;
        assert_eq!(context.get::<Config>(), Some(&Config(42)));
        assert_eq!(context.get_mut::<Database>(), None);
    }

    #[test]
    fn app_context_returns_registered_value() {
        let cx = CxTestBuilder::new()
            .app_context(Database("primary"))
            .build();

        let db: &Database = app_context(&cx);
        assert_eq!(db, &Database("primary"));
    }

    #[test]
    #[should_panic(expected = "attempted to access app context")]
    fn app_context_panics_for_unregistered_type() {
        let cx = Cx::default();
        let _: &Database = app_context(&cx);
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
    #[should_panic(expected = "attempted to access request context")]
    fn request_context_panics_for_unregistered_type() {
        let cx = Cx::default();
        let _: &Database = request_context(&cx);
    }
}
