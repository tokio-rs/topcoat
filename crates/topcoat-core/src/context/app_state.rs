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
