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

    /// Registers `value` under its concrete type `T`.
    ///
    /// # Panics
    ///
    /// Panics if a value of type `T` has already been registered.
    pub fn insert<T>(&mut self, value: T)
    where
        T: Any + Send + Sync,
    {
        assert!(
            self.entries.insert::<T>(value).is_none(),
            "duplicate context entry for type `{:?}`",
            type_name::<T>()
        );
    }

    /// Returns a reference to the registered value of type `T`, or `None` if
    /// no such value has been registered.
    ///
    /// This is an internal lookup used by [`app_context`] and [`request_context`]. Within a
    /// request, prefer the free functions rather than calling this method directly.
    fn get<T>(&self) -> Option<&T>
    where
        T: Any + Send + Sync,
    {
        self.entries.get::<T>()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

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
    #[should_panic(expected = "duplicate context entry")]
    fn register_panics_on_duplicate_type() {
        let mut context = ContextMap::new();
        context.insert(Database("primary"));
        context.insert(Database("replica"));
    }

    #[test]
    fn app_context_returns_registered_value() {
        let mut context = ContextMap::new();
        context.insert(Database("primary"));
        let cx = Cx::new(Arc::new(context), ContextMap::new());

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
        let mut context = ContextMap::new();
        context.insert(Database("primary"));
        let cx = Cx::new(Arc::new(ContextMap::new()), context);

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
