use std::sync::{Mutex, PoisonError};

use http::{HeaderMap, HeaderName, HeaderValue};
use topcoat_core::context::{Cx, request_context};

/// Headers that the router adds to the response for the current request,
/// whatever that response turns out to be.
///
/// A layer usually sets headers on the [`Response`](super::Response) it gets
/// back from [`Next::run`](crate::Next). When the handler returns an error
/// instead, there is no response yet, because the router only builds it once
/// the error has passed through every layer. Headers added here are applied
/// after the router has built the final response, for a success and an error
/// alike. The error itself stays unchanged, so outer layers can still inspect
/// it.
///
/// Get the headers of the current request with [`response_headers`].
///
/// # Examples
///
/// ```rust
/// use topcoat::{
///     context::Cx,
///     router::{Body, Layer, LayerFuture, Next, Path, header, response::response_headers},
/// };
///
/// struct RequestId;
///
/// impl Layer for RequestId {
///     fn path(&self) -> Option<&Path> {
///         None
///     }
///
///     fn handle<'a>(&'a self, cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
///         Box::pin(async move {
///             // Added to the 404 response for an unknown URL as well as to
///             // the response of a handler.
///             response_headers(cx).append(
///                 header::HeaderName::from_static("x-request-id"),
///                 header::HeaderValue::from_static("42"),
///             );
///             next.run(cx, body).await
///         })
///     }
/// }
/// ```
#[derive(Debug, Default)]
pub struct ResponseHeaders {
    pending: Mutex<HeaderMap>,
}

impl ResponseHeaders {
    /// Creates an empty set of pending headers.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `value` under `name` to the pending headers.
    ///
    /// The value is appended, so values that the response already has under
    /// `name` are kept. This lets a header that can repeat, like `Set-Cookie`,
    /// collect several values.
    pub fn append(&self, name: HeaderName, value: HeaderValue) {
        self.lock().append(name, value);
    }

    /// Adds every entry of `headers` to the pending headers, in the same way
    /// as [`append`](Self::append).
    pub fn extend(&self, headers: HeaderMap) {
        append_all(&mut self.lock(), headers);
    }

    /// Moves the pending headers onto `headers`, leaving the slot empty.
    pub(crate) fn apply(&self, headers: &mut HeaderMap) {
        let pending = std::mem::take(&mut *self.lock());
        append_all(headers, pending);
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HeaderMap> {
        self.pending.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Appends every entry of `from` to `into`, keeping the values `into` already
/// holds under the same names.
fn append_all(into: &mut HeaderMap, from: HeaderMap) {
    // Iterating a `HeaderMap` by value names a header on its first value only
    // and yields `None` for the values that follow it.
    let mut current = None;
    for (name, value) in from {
        if let Some(name) = name {
            current = Some(name);
        }
        let name = current
            .clone()
            .expect("a header map yields a name before its values");
        into.append(name, value);
    }
}

/// Returns the [`ResponseHeaders`] of the current request.
///
/// # Panics
///
/// Panics if the router is not handling the current request.
#[must_use]
#[track_caller]
pub fn response_headers(cx: &Cx) -> &ResponseHeaders {
    request_context::<ResponseHeaders>(cx)
}

#[cfg(test)]
mod tests {
    use http::header::{CACHE_CONTROL, SET_COOKIE};

    use super::*;

    fn values(headers: &HeaderMap, name: HeaderName) -> Vec<&str> {
        headers
            .get_all(name)
            .iter()
            .map(|value| value.to_str().unwrap())
            .collect()
    }

    #[test]
    fn appended_headers_are_added_next_to_existing_values() {
        let slot = ResponseHeaders::new();
        slot.append(SET_COOKIE, HeaderValue::from_static("a=1"));
        slot.append(SET_COOKIE, HeaderValue::from_static("b=2"));
        let mut headers = HeaderMap::new();
        headers.append(SET_COOKIE, HeaderValue::from_static("c=3"));

        slot.apply(&mut headers);

        assert_eq!(values(&headers, SET_COOKIE), ["c=3", "a=1", "b=2"]);
    }

    #[test]
    fn extending_queues_every_entry_of_a_map() {
        let slot = ResponseHeaders::new();
        let mut pending = HeaderMap::new();
        pending.append(SET_COOKIE, HeaderValue::from_static("a=1"));
        pending.append(SET_COOKIE, HeaderValue::from_static("b=2"));
        pending.append(CACHE_CONTROL, HeaderValue::from_static("no-store"));
        slot.extend(pending);
        let mut headers = HeaderMap::new();

        slot.apply(&mut headers);

        assert_eq!(values(&headers, SET_COOKIE), ["a=1", "b=2"]);
        assert_eq!(values(&headers, CACHE_CONTROL), ["no-store"]);
    }

    #[test]
    fn applying_drains_the_slot() {
        let slot = ResponseHeaders::new();
        slot.append(CACHE_CONTROL, HeaderValue::from_static("no-store"));
        let mut first = HeaderMap::new();
        let mut second = HeaderMap::new();

        slot.apply(&mut first);
        slot.apply(&mut second);

        assert_eq!(values(&first, CACHE_CONTROL), ["no-store"]);
        assert!(second.is_empty());
    }
}
