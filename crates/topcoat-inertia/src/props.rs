use std::sync::Mutex;

use serde::Serialize;
use topcoat_core::context::{Cx, try_request_context};
use topcoat_core::error::Result;

use crate::{Prop, ScrollMetadata, always, defer, lazy, merge, once, optional, scroll};

pub struct Props<'cx> {
    pub(crate) entries: Vec<(String, Prop<'cx>)>,
}

impl<'cx> Props<'cx> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn prop(&mut self, key: impl Into<String>, value: impl Serialize) -> &mut Self {
        self.prop_with(key, Prop::value(value))
    }

    pub fn lazy<T>(
        &mut self,
        key: impl Into<String>,
        future: impl Future<Output = Result<T>> + Send + 'cx,
    ) -> &mut Self
    where
        T: Serialize,
    {
        self.prop_with(key, lazy(future))
    }

    pub fn always(&mut self, key: impl Into<String>, value: impl Serialize) -> &mut Self {
        self.prop_with(key, always(value))
    }

    pub fn optional<T>(
        &mut self,
        key: impl Into<String>,
        future: impl Future<Output = Result<T>> + Send + 'cx,
    ) -> &mut Self
    where
        T: Serialize,
    {
        self.prop_with(key, optional(future))
    }

    pub fn defer<T>(
        &mut self,
        key: impl Into<String>,
        future: impl Future<Output = Result<T>> + Send + 'cx,
    ) -> &mut Self
    where
        T: Serialize,
    {
        self.prop_with(key, defer(future))
    }

    pub fn merge(&mut self, key: impl Into<String>, value: impl Serialize) -> &mut Self {
        self.prop_with(key, merge(value))
    }

    pub fn once<T>(
        &mut self,
        key: impl Into<String>,
        future: impl Future<Output = Result<T>> + Send + 'cx,
    ) -> &mut Self
    where
        T: Serialize,
    {
        self.prop_with(key, once(future))
    }

    pub fn scroll(
        &mut self,
        key: impl Into<String>,
        value: impl Serialize,
        metadata: ScrollMetadata,
    ) -> &mut Self {
        self.prop_with(key, scroll(value, metadata))
    }

    pub fn prop_with(&mut self, key: impl Into<String>, prop: Prop<'cx>) -> &mut Self {
        self.entries.push((key.into(), prop));
        self
    }
}

impl Default for Props<'_> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Default)]
pub(crate) struct SharedProps {
    entries: Mutex<Vec<(String, Prop<'static>)>>,
}

impl SharedProps {
    pub(crate) fn take(&self) -> Vec<(String, Prop<'static>)> {
        let mut entries = self
            .entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        std::mem::take(&mut *entries)
    }

    fn push(&self, key: String, prop: Prop<'static>) {
        self.entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push((key, prop));
    }
}

/// Adds an owned request-local shared prop.
///
/// # Errors
///
/// Returns an error when `value` cannot be serialized.
pub fn share(cx: &Cx, key: impl Into<String>, value: impl Serialize) -> Result<()> {
    let value = serde_json::to_value(value)?;
    share_with(cx, key, Prop::from_result(Ok(value)));
    Ok(())
}

pub fn share_with(cx: &Cx, key: impl Into<String>, prop: Prop<'static>) {
    if let Some(shared) = try_request_context::<SharedProps>(cx) {
        shared.push(key.into(), prop);
    }
}
