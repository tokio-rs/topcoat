use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::{Map, Value};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OnceMetadata {
    pub prop: String,
    pub expires_at: Option<u64>,
}

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

    #[must_use]
    pub fn previous_page(mut self, page: Option<u64>) -> Self {
        self.previous_page = page;
        self
    }

    #[must_use]
    pub fn next_page(mut self, page: Option<u64>) -> Self {
        self.next_page = page;
        self
    }

    #[must_use]
    pub fn current_page(mut self, page: u64) -> Self {
        self.current_page = page;
        self
    }

    #[must_use]
    pub fn reset(&self) -> bool {
        self.reset
    }

    pub(crate) fn set_reset(&mut self, reset: bool) {
        self.reset = reset;
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub component: String,
    pub props: Map<String, Value>,
    pub url: String,
    pub version: Option<String>,
    #[serde(skip_serializing_if = "is_false")]
    pub encrypt_history: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub clear_history: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub preserve_fragment: bool,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub deferred_props: BTreeMap<String, Vec<String>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub merge_props: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub prepend_props: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub deep_merge_props: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub match_props_on: Vec<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub once_props: BTreeMap<String, OnceMetadata>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub scroll_props: BTreeMap<String, ScrollMetadata>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rescued_props: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub shared_props: Vec<String>,
    #[serde(skip_serializing_if = "Map::is_empty")]
    pub flash: Map<String, Value>,
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_false(value: &bool) -> bool {
    !value
}
