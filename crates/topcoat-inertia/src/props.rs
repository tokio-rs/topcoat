use std::sync::Mutex;

use serde::Serialize;
use topcoat_core::context::{Cx, try_request_context};
use topcoat_core::error::Result;

use crate::{Prop, ScrollMetadata, always, defer, lazy, merge, once, optional, scroll};

/// A callback-scoped builder for shared Inertia props.
///
/// It mirrors [`Inertia`](crate::Inertia)'s prop methods and is passed to
/// [`InertiaConfig::share_with`](crate::InertiaConfig::share_with).
pub struct Props<'cx> {
    pub(crate) entries: Vec<(String, Prop<'cx>)>,
}

impl<'cx> Props<'cx> {
    /// Creates an empty shared-props builder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Adds an eagerly serialized prop.
    pub fn prop(&mut self, key: impl Into<String>, value: impl Serialize) -> &mut Self {
        self.prop_with(key, Prop::value(value))
    }

    /// Adds a future-backed prop.
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

    /// Adds a prop that bypasses partial-reload filtering.
    pub fn always(&mut self, key: impl Into<String>, value: impl Serialize) -> &mut Self {
        self.prop_with(key, always(value))
    }

    /// Adds a future omitted unless explicitly requested.
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

    /// Adds a future loaded after the initial page.
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

    /// Adds a prop appended on subsequent visits.
    pub fn merge(&mut self, key: impl Into<String>, value: impl Serialize) -> &mut Self {
        self.prop_with(key, merge(value))
    }

    /// Adds a future-backed once prop.
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

    /// Adds an infinite-scroll prop.
    pub fn scroll(
        &mut self,
        key: impl Into<String>,
        value: impl Serialize,
        metadata: ScrollMetadata,
    ) -> &mut Self {
        self.prop_with(key, scroll(value, metadata))
    }

    /// Adds a fully configured [`Prop`].
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

/// Adds an owned, request-local shared prop with custom behavior.
///
/// Borrowing futures belong in the configuration callback because request
/// context can store only `'static` props.
pub fn share_with(cx: &Cx, key: impl Into<String>, prop: Prop<'static>) {
    if let Some(shared) = try_request_context::<SharedProps>(cx) {
        shared.push(key.into(), prop);
    }
}
