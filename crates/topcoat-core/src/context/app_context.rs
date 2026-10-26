//! Type-keyed values registered once at startup and shared across every
//! request handled by the router.

use std::any::{Any, type_name};

use crate::context::{ContextMap, Cx};

/// Returns a reference to the app context value of type `T` registered on the
/// router, or `None` if no such value has been registered.
///
/// The requested type must exactly match the registered type.
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
    cx.scope.sealed().find_map(ContextMap::get::<T>)
}

/// Returns a reference to the app context value of type `T` registered on the
/// router.
///
/// The requested type must exactly match the registered type.
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
/// A temporary name for [`ContextMap`], kept until app and request context
/// are unified. Build it before serving requests, then share it across them.
/// Within a request, read values with [`app_context`] or [`try_app_context`].
pub type AppContext = ContextMap;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::CxTestBuilder;

    #[derive(Debug, PartialEq)]
    struct Database(&'static str);

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
