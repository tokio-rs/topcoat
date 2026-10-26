//! Values identified by type and shared within a request context scope.

use std::any::{Any, type_name};

use crate::context::{ContextMap, ContextRead, Cx};

/// Returns a reference to the request context value of type `T` registered on
/// the current request's [`Cx`], or `None` if no such value has been registered.
///
/// The requested type must exactly match the registered type. A child scope
/// can replace an inherited value without changing what its parent sees.
///
/// # Examples
///
/// ```rust
/// use topcoat::context::{Cx, try_request_context};
///
/// struct Customer;
///
/// fn current_customer(cx: &Cx) -> Option<&Customer> {
///     try_request_context(cx)
/// }
/// ```
#[must_use]
pub fn try_request_context<T>(cx: &Cx) -> Option<&T>
where
    T: Any + Send + Sync,
{
    let binding = cx.scope.values.lookup::<T>();
    if let Some(tracker) = &cx.tracker {
        tracker.record(ContextRead::new::<T>(binding.map(|(id, _)| id)));
    }
    binding.map(|(_, value)| value)
}

/// Returns a reference to the request context value of type `T` registered on
/// the current request's [`Cx`].
///
/// The requested type must exactly match the registered type. A child scope
/// can replace an inherited value without changing what its parent sees.
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
#[must_use]
#[track_caller]
pub fn request_context<T>(cx: &Cx) -> &T
where
    T: Any + Send + Sync,
{
    match try_request_context(cx) {
        Some(value) => value,
        None => panic!(
            "attempted to access request context of type `{:?}`, but this type was not registered for this context",
            type_name::<T>()
        ),
    }
}

/// The values registered for one scope of a request.
///
/// A temporary name for [`ContextMap`], kept until app and request context
/// are unified.
pub type RequestContext = ContextMap;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::CxTestBuilder;

    #[derive(Debug, PartialEq)]
    struct Database(&'static str);

    #[test]
    fn request_context_returns_registered_value() {
        let cx = CxTestBuilder::new()
            .request_context(Database("primary"))
            .build();

        let db: &Database = request_context(&cx);
        assert_eq!(db, &Database("primary"));
    }

    #[test]
    fn try_request_context_returns_registered_value() {
        let cx = CxTestBuilder::new()
            .request_context(Database("primary"))
            .build();

        assert_eq!(
            try_request_context::<Database>(&cx),
            Some(&Database("primary"))
        );
    }

    #[test]
    fn try_request_context_returns_none_for_unregistered_type() {
        let cx = Cx::default();
        assert_eq!(try_request_context::<Database>(&cx), None);
    }

    #[test]
    #[should_panic(expected = "attempted to access request context")]
    fn request_context_panics_for_unregistered_type() {
        let cx = Cx::default();
        let _: &Database = request_context(&cx);
    }
}
