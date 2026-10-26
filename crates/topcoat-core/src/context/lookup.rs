//! Reading context values by type.

use std::any::{Any, type_name};

use crate::context::{ContextRead, Cx};

/// Returns a reference to the context value of type `T` visible to `cx`, or
/// `None` if no such value has been registered.
///
/// The requested type must exactly match the registered type. Values added
/// with [`Cx::with`] shadow values of the same type registered earlier, such
/// as on the router, without changing what the parent scope sees.
///
/// # Examples
///
/// ```rust
/// use topcoat::context::{Cx, try_context};
///
/// struct Customer;
///
/// fn current_customer(cx: &Cx) -> Option<&Customer> {
///     try_context(cx)
/// }
/// ```
#[must_use]
pub fn try_context<T>(cx: &Cx) -> Option<&T>
where
    T: Any + Send + Sync,
{
    let binding = cx.scope.lookup::<T>();
    if let Some(tracker) = &cx.tracker {
        tracker.record(ContextRead::new::<T>(binding.map(|(id, _)| id)));
    }
    binding.map(|(_, value)| value)
}

/// Returns a reference to the context value of type `T` visible to `cx`.
///
/// The requested type must exactly match the registered type. Values added
/// with [`Cx::with`] shadow values of the same type registered earlier, such
/// as on the router, without changing what the parent scope sees.
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
/// use topcoat::context::{Cx, context};
///
/// struct Database {/* ... */}
///
/// async fn load_user(cx: &Cx, id: u64) -> User {
///     let db: &Database = context(cx);
///     db.fetch_user(id).await
/// }
/// ```
#[must_use]
#[track_caller]
pub fn context<T>(cx: &Cx) -> &T
where
    T: Any + Send + Sync,
{
    match try_context(cx) {
        Some(value) => value,
        None => panic!(
            "attempted to access context of type `{:?}`, but this type was not registered for this context",
            type_name::<T>()
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::context::ContextMap;

    #[derive(Debug, PartialEq)]
    struct Database(&'static str);

    /// Returns a handle whose sealed app layer holds `value`.
    fn with_app<T: Any + Send + Sync>(value: T) -> Cx {
        let mut app = ContextMap::new();
        app.insert(value);
        Cx::new(Arc::new(app))
    }

    #[test]
    fn context_returns_a_request_value() {
        let cx = Cx::default().with(Database("primary"));
        assert_eq!(context::<Database>(&cx), &Database("primary"));
    }

    #[test]
    fn context_returns_an_app_value() {
        let cx = with_app(Database("primary"));
        assert_eq!(context::<Database>(&cx), &Database("primary"));
    }

    #[test]
    fn a_request_value_shadows_an_app_value() {
        let cx = with_app(Database("primary"));
        let child = cx.with(Database("replica"));

        assert_eq!(context::<Database>(&cx), &Database("primary"));
        assert_eq!(context::<Database>(&child), &Database("replica"));
    }

    #[test]
    fn try_context_returns_none_for_unregistered_type() {
        let cx = Cx::default();
        assert_eq!(try_context::<Database>(&cx), None);
    }

    #[test]
    #[should_panic(expected = "attempted to access context")]
    fn context_panics_for_unregistered_type() {
        let cx = Cx::default();
        let _: &Database = context(&cx);
    }
}
