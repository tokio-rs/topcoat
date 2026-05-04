//! Process-wide values shared across every request.
//!
//! [`AppState`] is a type-keyed map of values registered once at startup and
//! made available to every request handled by the router. Within a request,
//! [`app_state`] retrieves a reference to a registered value by its type.

use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

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
    match cx.state.get::<T>() {
        Some(value) => value,
        None => panic!(
            "attempted to access app state of type `{:?}`, but this type was not registered for this context",
            TypeId::of::<T>()
        ),
    }
}

/// A type-keyed container of values shared across every request.
///
/// Each registered value is stored under its [`TypeId`], so a given type can
/// only be registered once. Values are retrieved within a request via
/// [`app_state`].
#[derive(Default, Debug)]
pub struct AppState {
    entries: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl AppState {
    /// Creates an empty `AppState`.
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
        if self
            .entries
            .insert(TypeId::of::<T>(), Box::new(value))
            .is_some()
        {
            panic!("duplicate state entry for type `{:?}`", TypeId::of::<T>())
        }
    }

    /// Returns a reference to the registered value of type `T`, or `None` if
    /// no such value has been registered.
    ///
    /// This is an internal lookup used by [`app_state`]. Within a request,
    /// prefer the [`app_state`] free function rather than calling this method
    /// directly.
    fn get<T>(&self) -> Option<&T>
    where
        T: Any + Send + Sync,
    {
        self.entries.get(&TypeId::of::<T>()).as_ref().map(|value| {
            value
                .downcast_ref()
                .expect("value must downcast to the type it was registered as")
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::context::{AbortStore, MemoizeCache};

    #[derive(Debug, PartialEq)]
    struct Database(&'static str);

    #[derive(Debug, PartialEq)]
    struct Config(u32);

    fn cx_with(state: AppState) -> Cx {
        Cx {
            id: super::super::CxId(0),
            state: Arc::new(state),
            parts: http::Request::new(()).into_parts().0,
            cache: MemoizeCache::new(),
            abort: AbortStore::new(),
        }
    }

    #[test]
    fn register_and_get_returns_value() {
        let mut state = AppState::new();
        state.register(Database("primary"));

        assert_eq!(state.get::<Database>(), Some(&Database("primary")));
    }

    #[test]
    fn get_returns_none_for_unregistered_type() {
        let state = AppState::new();
        assert_eq!(state.get::<Database>(), None);
    }

    #[test]
    fn multiple_types_coexist() {
        let mut state = AppState::new();
        state.register(Database("primary"));
        state.register(Config(42));

        assert_eq!(state.get::<Database>(), Some(&Database("primary")));
        assert_eq!(state.get::<Config>(), Some(&Config(42)));
    }

    #[test]
    #[should_panic(expected = "duplicate state entry")]
    fn register_panics_on_duplicate_type() {
        let mut state = AppState::new();
        state.register(Database("primary"));
        state.register(Database("replica"));
    }

    #[test]
    fn app_state_returns_registered_value() {
        let mut state = AppState::new();
        state.register(Database("primary"));
        let cx = cx_with(state);

        let db: &Database = app_state(&cx);
        assert_eq!(db, &Database("primary"));
    }

    #[test]
    #[should_panic(expected = "attempted to access app state")]
    fn app_state_panics_for_unregistered_type() {
        let cx = cx_with(AppState::new());
        let _: &Database = app_state(&cx);
    }
}
