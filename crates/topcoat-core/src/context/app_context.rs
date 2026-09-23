//! Values registered once at startup, looked up by type and shared by every
//! request the router handles.

use std::any::{Any, type_name};

use crate::context::Cx;

/// Returns the app context value of type `T` registered on the router, or
/// `None` if no such value is registered.
///
/// Values are looked up by type, so the app context holds at most one value
/// per type. Register values on the router builder with `.app_context(value)`.
///
/// # Examples
///
/// ```rust
/// use topcoat::context::{Cx, try_app_context};
///
/// struct FeatureConfig;
///
/// fn feature_config(cx: &Cx) -> Option<&FeatureConfig> {
///     try_app_context(cx)
/// }
/// ```
#[must_use]
pub fn try_app_context<T>(cx: &Cx) -> Option<&T>
where
    T: Any + Send + Sync,
{
    cx.state.shared.app_context.get::<T>()
}

/// Returns the app context value of type `T` registered on the router.
///
/// This is [`try_app_context`] for values that must be present.
///
/// # Panics
///
/// Panics if no value of type `T` is registered.
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
#[must_use]
#[track_caller]
pub fn app_context<T>(cx: &Cx) -> &T
where
    T: Any + Send + Sync,
{
    match try_app_context(cx) {
        Some(value) => value,
        None => panic!(
            "attempted to access app context of type `{:?}`, but this type was not registered for this context",
            type_name::<T>()
        ),
    }
}

/// The values shared by every request, looked up by type.
///
/// An `AppContext` holds at most one value per type. It is built once at
/// startup and then shared, read-only, by every request the router handles.
/// Within a request, read values with [`app_context`] or [`try_app_context`].
#[derive(Default, Debug)]
pub struct AppContext {
    entries: anymap3::Map<dyn Any + Send + Sync>,
}

impl AppContext {
    /// Creates an empty `AppContext`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers `value` under its type `T`.
    ///
    /// Registering a type that is already present replaces the previous value
    /// and returns it.
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
    /// Within a request, use [`app_context`] or [`try_app_context`] instead.
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
    use crate::context::CxTestBuilder;

    #[derive(Debug, PartialEq)]
    struct Database(&'static str);

    #[derive(Debug, PartialEq)]
    struct Config(u32);

    #[test]
    fn register_and_get_returns_value() {
        let mut context = AppContext::new();
        context.insert(Database("primary"));

        assert_eq!(context.get::<Database>(), Some(&Database("primary")));
    }

    #[test]
    fn get_returns_none_for_unregistered_type() {
        let context = AppContext::new();
        assert_eq!(context.get::<Database>(), None);
    }

    #[test]
    fn multiple_types_coexist() {
        let mut context = AppContext::new();
        context.insert(Database("primary"));
        context.insert(Config(42));

        assert_eq!(context.get::<Database>(), Some(&Database("primary")));
        assert_eq!(context.get::<Config>(), Some(&Config(42)));
    }

    #[test]
    fn insert_replaces_and_returns_the_displaced_value() {
        let mut context = AppContext::new();
        assert_eq!(context.insert(Database("primary")), None);
        assert_eq!(
            context.insert(Database("replica")),
            Some(Database("primary"))
        );
        assert_eq!(context.get::<Database>(), Some(&Database("replica")));
    }

    #[test]
    fn contains_reports_registered_types() {
        let mut context = AppContext::new();
        assert!(!context.contains::<Database>());
        context.insert(Database("primary"));
        assert!(context.contains::<Database>());
        assert!(!context.contains::<Config>());
    }

    #[test]
    fn get_mut_allows_mutation_in_place() {
        let mut context = AppContext::new();
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
    fn try_app_context_returns_registered_value() {
        let cx = CxTestBuilder::new()
            .app_context(Database("primary"))
            .build();

        assert_eq!(try_app_context::<Database>(&cx), Some(&Database("primary")));
    }

    #[test]
    fn try_app_context_returns_none_for_unregistered_type() {
        let cx = Cx::default();
        assert_eq!(try_app_context::<Database>(&cx), None);
    }

    #[test]
    #[should_panic(expected = "attempted to access app context")]
    fn app_context_panics_for_unregistered_type() {
        let cx = Cx::default();
        let _: &Database = app_context(&cx);
    }
}
