use http::HeaderMap;
use topcoat_core::context::{Cx, try_request_context};

use crate::header;

/// The direction requested by an Inertia.js infinite-scroll visit.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum MergeIntent {
    /// Add the new page after the existing collection.
    #[default]
    Append,
    /// Add the new page before the existing collection.
    Prepend,
}

/// Parsed Inertia.js request metadata.
///
/// The router layer creates this from the protocol headers and stores it in
/// request context. Most handlers only need the free request helpers.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct InertiaRequest {
    inertia: bool,
    version: Option<String>,
    partial_component: Option<String>,
    only: Option<Vec<String>>,
    except: Option<Vec<String>>,
    reset: Vec<String>,
    except_once: Vec<String>,
    error_bag: Option<String>,
    scroll_intent: MergeIntent,
    prefetch: bool,
}

impl InertiaRequest {
    /// Parses all supported v3 request headers.
    ///
    /// Invalid UTF-8 and empty optional values are treated as absent. Unknown
    /// scroll intents fall back to append.
    #[must_use]
    pub fn from_headers(headers: &HeaderMap) -> Self {
        Self {
            inertia: headers.contains_key(&header::X_INERTIA),
            version: string(headers, &header::X_INERTIA_VERSION),
            partial_component: string(headers, &header::X_INERTIA_PARTIAL_COMPONENT),
            only: list(headers, &header::X_INERTIA_PARTIAL_DATA),
            except: list(headers, &header::X_INERTIA_PARTIAL_EXCEPT),
            reset: list(headers, &header::X_INERTIA_RESET).unwrap_or_default(),
            except_once: list(headers, &header::X_INERTIA_EXCEPT_ONCE_PROPS).unwrap_or_default(),
            error_bag: string(headers, &header::X_INERTIA_ERROR_BAG),
            scroll_intent: match string(headers, &header::X_INERTIA_INFINITE_SCROLL_MERGE_INTENT)
                .as_deref()
            {
                Some(value) if value.eq_ignore_ascii_case("prepend") => MergeIntent::Prepend,
                _ => MergeIntent::Append,
            },
            prefetch: string(headers, &header::PURPOSE)
                .is_some_and(|value| value.eq_ignore_ascii_case("prefetch")),
        }
    }

    /// Returns whether `X-Inertia` was present.
    #[must_use]
    pub fn is_inertia(&self) -> bool {
        self.inertia
    }

    /// Returns the client asset version.
    #[must_use]
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    /// Returns the component named by a partial reload.
    #[must_use]
    pub fn partial_component(&self) -> Option<&str> {
        self.partial_component.as_deref()
    }

    /// Returns the partial prop paths selected with `only`.
    #[must_use]
    pub fn only(&self) -> Option<&[String]> {
        self.only.as_deref()
    }

    /// Returns the partial prop paths selected with `except`.
    #[must_use]
    pub fn except(&self) -> Option<&[String]> {
        self.except.as_deref()
    }

    /// Returns prop paths whose client merge state must reset.
    #[must_use]
    pub fn reset(&self) -> &[String] {
        &self.reset
    }

    /// Returns once-prop keys already held by the client.
    #[must_use]
    pub fn except_once(&self) -> &[String] {
        &self.except_once
    }

    /// Returns the selected validation error bag.
    #[must_use]
    pub fn error_bag(&self) -> Option<&str> {
        self.error_bag.as_deref()
    }

    /// Returns the infinite-scroll merge direction.
    #[must_use]
    pub fn scroll_intent(&self) -> MergeIntent {
        self.scroll_intent
    }

    /// Returns whether the browser marked this as a speculative prefetch.
    #[must_use]
    pub fn is_prefetch(&self) -> bool {
        self.prefetch
    }

    #[must_use]
    pub(crate) fn is_partial_for(&self, component: &str) -> bool {
        self.inertia && self.partial_component.as_deref() == Some(component)
    }
}

fn string(headers: &HeaderMap, name: &http::HeaderName) -> Option<String> {
    headers
        .get(name)?
        .to_str()
        .ok()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn list(headers: &HeaderMap, name: &http::HeaderName) -> Option<Vec<String>> {
    let values = headers
        .get(name)?
        .to_str()
        .ok()?
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    (!values.is_empty()).then_some(values)
}

fn request(cx: &Cx) -> Option<&InertiaRequest> {
    try_request_context(cx)
}

#[must_use]
/// Returns whether the current request is an Inertia visit.
pub fn inertia_request(cx: &Cx) -> bool {
    request(cx).is_some_and(InertiaRequest::is_inertia)
}

#[must_use]
/// Returns the partial component named by the current request.
pub fn inertia_partial_component(cx: &Cx) -> Option<&str> {
    request(cx).and_then(InertiaRequest::partial_component)
}

#[must_use]
/// Returns whether the current request is an Inertia prefetch.
pub fn inertia_prefetch(cx: &Cx) -> bool {
    request(cx).is_some_and(InertiaRequest::is_prefetch)
}

#[cfg(test)]
mod tests {
    use http::HeaderValue;

    use super::*;

    #[test]
    fn parses_protocol_headers() {
        let mut headers = HeaderMap::new();
        headers.insert(&header::X_INERTIA, HeaderValue::from_static("anything"));
        headers.insert(
            &header::X_INERTIA_PARTIAL_DATA,
            HeaderValue::from_static(" users, , stats "),
        );
        headers.insert(
            &header::X_INERTIA_INFINITE_SCROLL_MERGE_INTENT,
            HeaderValue::from_static("prepend"),
        );
        headers.insert(&header::PURPOSE, HeaderValue::from_static("PreFetch"));

        let request = InertiaRequest::from_headers(&headers);

        assert!(request.is_inertia());
        assert_eq!(
            request.only(),
            Some(&["users".to_owned(), "stats".to_owned()][..])
        );
        assert_eq!(request.scroll_intent(), MergeIntent::Prepend);
        assert!(request.is_prefetch());
    }

    #[test]
    fn ignores_invalid_and_empty_optional_headers() {
        let mut headers = HeaderMap::new();
        headers.insert(
            &header::X_INERTIA_VERSION,
            HeaderValue::from_bytes(b"\xff").unwrap(),
        );
        headers.insert(
            &header::X_INERTIA_PARTIAL_DATA,
            HeaderValue::from_static(" , "),
        );
        headers.insert(
            &header::X_INERTIA_INFINITE_SCROLL_MERGE_INTENT,
            HeaderValue::from_static("sideways"),
        );

        let request = InertiaRequest::from_headers(&headers);

        assert_eq!(request.version(), None);
        assert_eq!(request.only(), None);
        assert_eq!(request.scroll_intent(), MergeIntent::Append);
    }
}
