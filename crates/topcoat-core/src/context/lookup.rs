//! Helpers for borrowing values from a context.

use std::any::{Any, type_name};

use crate::context::{ContextRead, Cx};

/// Borrows the value of type `T` in `cx`, or returns `None` if it is absent.
///
/// Ask for the exact type that was registered. A value added with
/// [`Cx::with`] overrides an earlier value of that type, including a router
/// default. Only the child context sees the replacement.
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

/// Borrows the value of type `T` in `cx`.
///
/// Use the same type that was registered. Values added with [`Cx::with`]
/// take priority over earlier values of that type in the child context.
/// The parent still reads its original value.
///
/// # Panics
///
/// Panics if `cx` has no value of type `T`.
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

    #[test]
    fn context_returns_a_sealed_value() {
        let mut shared = ContextMap::new();
        shared.insert(Database("primary"));
        let cx = Cx::new(Arc::new(shared));

        assert_eq!(context::<Database>(&cx), &Database("primary"));
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
