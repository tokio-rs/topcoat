//! Type-keyed values registered once at startup and shared across every
//! request handled by the router.

use std::any::{Any, type_name};

use crate::context::Cx;

/// Returns a reference to the app context value of type `T` registered on the
/// router, or `None` if no such value has been registered.
///
/// The lookup is keyed by `T`'s [`TypeId`](std::any::TypeId), so each type may
/// have at most one registered value.
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
    cx.shared.app_context.get::<T>()
}

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

/// The type-keyed values shared by every request.
///
/// Each registered value is stored under its [`TypeId`](std::any::TypeId), so a
/// given type can only be registered once. An `AppContext` is assembled once at
/// startup and then shared read-only across every request handled by the
/// router; within a request, values are retrieved with [`app_context`] or
/// [`try_app_context`].
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
    /// Within a request, prefer the [`app_context`] and [`try_app_context`]
    /// free functions over reaching for this directly.
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
