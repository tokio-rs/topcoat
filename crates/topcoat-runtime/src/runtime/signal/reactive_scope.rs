use serde::Serialize;
use topcoat_core::runtime::{context::Cx, error::Error};
use topcoat_view::runtime::{NodeViewParts, Unescaped, View, ViewParts};
use uuid::Uuid;

use crate::runtime::{Shard, SignalId, Signals};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct ReactiveScopeId(Uuid);

impl ReactiveScopeId {
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ReactiveScopeId {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

pub struct ReactiveScope {
    id: ReactiveScopeId,
    track: Vec<SignalId>,
    path: String,
    placeholder: View,
}

impl ReactiveScope {
    /// Constructs a new reactive scope, recording the tracked signals and
    /// rendering its placeholder shard.
    ///
    /// # Errors
    ///
    /// Propagates any error produced while rendering the shard.
    #[inline]
    pub async fn new<S>(cx: &Cx, signals: S, shard: Shard<S>) -> Result<Self, Error>
    where
        S: Signals,
    {
        Ok(Self {
            id: ReactiveScopeId::new(),
            track: signals.ids().collect(),
            path: "/_topcoat/shards/".to_owned() + shard.id().as_str(),
            placeholder: shard.render(cx, signals).await?,
        })
    }
}

impl NodeViewParts for ReactiveScope {
    fn into_view_parts(self, parts: &mut ViewParts) {
        parts.push(Unescaped::new_unchecked("<!-- ::topcoat::scope::start("));
        parts.push(Unescaped::new_unchecked(
            serde_json::to_string(&self.id).unwrap(),
        ));
        parts.push(Unescaped::new_unchecked(", "));
        parts.push(Unescaped::new_unchecked(
            serde_json::to_string(&self.track).unwrap(),
        ));
        parts.push(Unescaped::new_unchecked(", "));
        parts.push(Unescaped::new_unchecked(
            serde_json::to_string(&self.path).unwrap(),
        ));
        parts.push(Unescaped::new_unchecked(") -->"));
        parts.push(self.placeholder);
        parts.push(Unescaped::new_unchecked("<!-- ::topcoat::scope::end("));
        parts.push(Unescaped::new_unchecked(
            serde_json::to_string(&self.id).unwrap(),
        ));
        parts.push(Unescaped::new_unchecked(") -->"));
    }
}
