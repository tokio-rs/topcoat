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

/// Visits every value of type `T` in `cx`, from the closest to the oldest.
///
/// Adding a value with [`Cx::with`] shadows the earlier values of its type
/// but keeps them. [`context`] returns only the closest value. This
/// function starts at that value and walks back through every value it
/// shadows, in the reverse of the order they were added.
///
/// # Examples
///
/// ```rust
/// use topcoat::context::{Cx, iter_context};
///
/// struct Crumb(&'static str);
///
/// let cx = Cx::default().with(Crumb("Home")).with(Crumb("Posts"));
/// let crumbs: Vec<_> = iter_context::<Crumb>(&cx).map(|crumb| crumb.0).collect();
///
/// assert_eq!(crumbs, ["Posts", "Home"]);
/// ```
pub fn iter_context<T>(cx: &Cx) -> impl Iterator<Item = &T>
where
    T: Any + Send + Sync,
{
    if let Some(tracker) = &cx.tracker {
        tracker.record(ContextRead::all::<T>(&cx.scope));
    }
    cx.scope.iter::<T>()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::context::{ContextMap, memoize_cache};

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

    #[test]
    fn iter_context_visits_request_values_before_sealed_values() {
        let mut shared = ContextMap::new();
        shared.insert(Database("primary"));
        let cx = Cx::new(Arc::new(shared))
            .with(Database("replica"))
            .with(Database("cache"));

        assert_eq!(
            iter_context::<Database>(&cx).collect::<Vec<_>>(),
            [
                &Database("cache"),
                &Database("replica"),
                &Database("primary")
            ]
        );
    }

    #[test]
    fn memoized_iterations_rerun_when_a_value_is_added() {
        fn count(cx: Cx, (): ()) -> usize {
            iter_context::<Database>(&cx).count()
        }

        let cx = Cx::default().with(Database("primary"));
        let child = cx.with(Database("replica"));

        assert_eq!(*memoize_cache(&cx).memoize(&cx, (), (), count), 1);
        assert_eq!(*memoize_cache(&child).memoize(&child, (), (), count), 2);
        assert_eq!(*memoize_cache(&cx).memoize(&cx, (), (), count), 1);
    }

    #[test]
    fn memoized_iterations_through_own_values_depend_on_the_caller() {
        fn count(cx: Cx, (): ()) -> usize {
            iter_context::<Database>(&cx.with(Database("own"))).count()
        }

        let cx = Cx::default().with(Database("primary"));
        let child = cx.with(Database("replica"));

        assert_eq!(*memoize_cache(&cx).memoize(&cx, (), (), count), 2);
        assert_eq!(*memoize_cache(&child).memoize(&child, (), (), count), 3);
    }
}
