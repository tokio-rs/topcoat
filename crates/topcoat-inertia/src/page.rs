use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::{Map, Value};

/// Client cache metadata for a once prop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OnceMetadata {
    /// The page prop path represented by this cache entry.
    pub prop: String,
    /// Absolute Unix expiry time in milliseconds, when configured.
    pub expires_at: Option<u64>,
}

/// Pagination metadata used by Inertia.js infinite scroll.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScrollMetadata {
    page_name: String,
    previous_page: Option<u64>,
    next_page: Option<u64>,
    current_page: u64,
    reset: bool,
}

impl ScrollMetadata {
    /// Creates metadata for the request query parameter named `page_name`.
    #[must_use]
    pub fn new(page_name: impl Into<String>) -> Self {
        Self {
            page_name: page_name.into(),
            previous_page: None,
            next_page: None,
            current_page: 1,
            reset: false,
        }
    }

    /// Sets the previous page number, or `None` at the beginning.
    #[must_use]
    pub fn previous_page(mut self, page: Option<u64>) -> Self {
        self.previous_page = page;
        self
    }

    /// Sets the next page number, or `None` at the end.
    #[must_use]
    pub fn next_page(mut self, page: Option<u64>) -> Self {
        self.next_page = page;
        self
    }

    /// Sets the current page number. It defaults to `1`.
    #[must_use]
    pub fn current_page(mut self, page: u64) -> Self {
        self.current_page = page;
        self
    }

    /// Returns whether the current request resets client merge state.
    #[must_use]
    pub fn reset(&self) -> bool {
        self.reset
    }

    pub(crate) fn set_reset(&mut self, reset: bool) {
        self.reset = reset;
    }
}

/// The Inertia.js v3 page object sent to the client.
///
/// Applications normally build this through [`Inertia`](crate::Inertia).
/// Fields map directly to the v3 page protocol and empty metadata is omitted.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    /// The client component name.
    pub component: String,
    /// Resolved page props, including the reserved `errors` object.
    pub props: Map<String, Value>,
    /// The current page URL.
    pub url: String,
    /// The server asset version, when versioning is enabled.
    pub version: Option<String>,
    #[serde(skip_serializing_if = "is_false")]
    /// Whether browser history state is encrypted.
    pub encrypt_history: bool,
    #[serde(skip_serializing_if = "is_false")]
    /// Whether the client clears its existing history entries.
    pub clear_history: bool,
    #[serde(skip_serializing_if = "is_false")]
    /// Whether the client preserves the current URL fragment.
    pub preserve_fragment: bool,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    /// Deferred prop paths grouped by follow-up request name.
    pub deferred_props: BTreeMap<String, Vec<String>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    /// Prop paths appended on subsequent visits.
    pub merge_props: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    /// Prop paths prepended on subsequent visits.
    pub prepend_props: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    /// Prop paths deep-merged on subsequent visits.
    pub deep_merge_props: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    /// Nested item paths used to match merged collection entries.
    pub match_props_on: Vec<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    /// Once-prop cache metadata keyed by client cache key.
    pub once_props: BTreeMap<String, OnceMetadata>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    /// Infinite-scroll metadata keyed by prop path.
    pub scroll_props: BTreeMap<String, ScrollMetadata>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    /// Deferred paths omitted because their rescued resolver failed.
    pub rescued_props: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    /// Top-level prop keys declared through shared-prop sources.
    pub shared_props: Vec<String>,
    #[serde(skip_serializing_if = "Map::is_empty")]
    /// One-time page data kept outside browser history props.
    pub flash: Map<String, Value>,
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_false(value: &bool) -> bool {
    !value
}
