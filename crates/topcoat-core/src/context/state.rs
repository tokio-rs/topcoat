//! Type-keyed values made available through the request context.
//!
//! [`State`] is a type-keyed map of values, looked up by their [`TypeId`].
//! Each [`Cx`] carries two of them:
//!
//! - **App state** is registered once at startup and shared across every
//!   request handled by the router. Within a request, [`app_state`] retrieves
//!   a reference to a registered value by its type.
//! - **Request state** is scoped to a single request and dropped when the
//!   request ends. Within a request, [`request_state`] retrieves a reference
//!   to a registered value by its type.

use std::any::{Any, TypeId};

use crate::context::Cx;

/// Returns a reference to the app state value of type `T` registered on the
/// router.
///
/// The lookup is keyed by `T`'s [`TypeId`], so each type may have at most one
/// registered value.
///
/// # Panics
///
/// Panics if no value of type `T` has been registered.
///
/// # Examples
///
/// ```rust,ignore
/// use topcoat::context::{Cx, app_state};
///
/// struct Database { /* ... */ }
///
/// async fn load_user(cx: &Cx, id: u64) -> User {
///     let db: &Database = app_state(cx);
///     db.fetch_user(id).await
/// }
/// ```
pub fn app_state<T>(cx: &Cx) -> &T
where
    T: Any + Send + Sync,
{
    match cx.app_state.get::<T>() {
        Some(value) => value,
        None => panic!(
            "attempted to access app state of type `{:?}`, but this type was not registered for this context",
            TypeId::of::<T>()
        ),
    }
}

/// Returns a reference to the request state value of type `T` registered on
/// the current request's [`Cx`].
///
/// The lookup is keyed by `T`'s [`TypeId`], so each type may have at most one
/// registered value per request. Request state lives only for the duration of
/// the request that owns it; once the request completes, every value is
/// dropped.
///
/// # Panics
///
/// Panics if no value of type `T` has been registered on this request's `Cx`.
///
/// # Examples
///
/// ```rust,ignore
/// use topcoat::context::{Cx, request_state};
///
/// struct RequestId(String);
///
/// async fn current_request_id(cx: &Cx) -> &str {
///     let id: &RequestId = request_state(cx);
///     &id.0
/// }
/// ```
pub fn request_state<T>(cx: &Cx) -> &T
where
    T: Any + Send + Sync,
{
    match cx.request_state.get::<T>() {
        Some(value) => value,
        None => panic!(
            "attempted to access request state of type `{:?}`, but this type was not registered for this context",
            TypeId::of::<T>()
        ),
    }
}

/// A type-keyed container of values.
///
/// Each registered value is stored under its [`TypeId`], so a given type can
/// only be registered once per `State`. Used by [`Cx`] to hold both the
/// router-wide app state and the per-request request state; values are
/// retrieved within a request via [`app_state`] or [`request_state`].
#[derive(Default, Debug)]
pub struct State {
    entries: anymap3::Map<dyn Any + Send + Sync>,
}

impl State {
    /// Creates an empty `State`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers `value` under its concrete type `T`.
    ///
    /// # Panics
    ///
    /// Panics if a value of type `T` has already been registered.
    pub fn register<T>(&mut self, value: T)
    where
        T: Any + Send + Sync,
    {
        if self.entries.insert::<T>(value).is_some() {
            panic!("duplicate state entry for type `{:?}`", TypeId::of::<T>())
        }
    }

    /// Returns a reference to the registered value of type `T`, or `None` if
    /// no such value has been registered.
    ///
    /// This is an internal lookup used by [`app_state`] and [`request_state`]. Within a request,
    /// prefer the free functions rather than calling this method directly.
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
        let mut state = State::new();
        state.register(Database("primary"));

        assert_eq!(state.get::<Database>(), Some(&Database("primary")));
    }

    #[test]
    fn get_returns_none_for_unregistered_type() {
        let state = State::new();
        assert_eq!(state.get::<Database>(), None);
    }

    #[test]
    fn multiple_types_coexist() {
        let mut state = State::new();
        state.register(Database("primary"));
        state.register(Config(42));

        assert_eq!(state.get::<Database>(), Some(&Database("primary")));
        assert_eq!(state.get::<Config>(), Some(&Config(42)));
    }

    #[test]
    #[should_panic(expected = "duplicate state entry")]
    fn register_panics_on_duplicate_type() {
        let mut state = State::new();
        state.register(Database("primary"));
        state.register(Database("replica"));
    }

    #[test]
    fn app_state_returns_registered_value() {
        let mut state = State::new();
        state.register(Database("primary"));
        let cx = Cx::new(Arc::new(state), State::new());

        let db: &Database = app_state(&cx);
        assert_eq!(db, &Database("primary"));
    }

    #[test]
    #[should_panic(expected = "attempted to access app state")]
    fn app_state_panics_for_unregistered_type() {
        let cx = Cx::default();
        let _: &Database = app_state(&cx);
    }

    #[test]
    fn request_state_returns_registered_value() {
        let mut state = State::new();
        state.register(Database("primary"));
        let cx = Cx::new(Arc::new(State::new()), state);

        let db: &Database = request_state(&cx);
        assert_eq!(db, &Database("primary"));
    }

    #[test]
    #[should_panic(expected = "attempted to access request state")]
    fn request_state_panics_for_unregistered_type() {
        let cx = Cx::default();
        let _: &Database = request_state(&cx);
    }
}
