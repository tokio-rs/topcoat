use serde::Serialize;
use topcoat_core::context::Cx;
use topcoat_view::runtime::{IntoViewParts, Unescaped, View, ViewPart};
use uuid::Uuid;

use crate::runtime::{Island, SignalId, Signals};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct ReactiveScopeId(Uuid);

impl ReactiveScopeId {
    #[inline]
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
    #[inline]
    pub async fn new<S, E>(cx: &Cx, signals: S, island: Island<S, E>) -> Result<Self, E>
    where
        S: Signals,
    {
        Ok(Self {
            id: ReactiveScopeId::new(),
            track: signals.ids().collect(),
            path: "/_topcoat/islands/".to_owned() + island.id().as_str(),
            placeholder: island.render(cx, signals).await?,
        })
    }
}

impl IntoViewParts for ReactiveScope {
    fn into_view_parts(self) -> impl Iterator<Item = ViewPart> {
        [
            ViewPart::UnescapedStaticStr(Unescaped::new_unchecked("<!-- ::topcoat::scope::start(")),
            ViewPart::UnescapedString(Unescaped::new_unchecked(
                serde_json::to_string(&self.id).unwrap(),
            )),
            ViewPart::UnescapedStaticStr(Unescaped::new_unchecked(", ")),
            ViewPart::UnescapedString(Unescaped::new_unchecked(
                serde_json::to_string(&self.track).unwrap(),
            )),
            ViewPart::UnescapedStaticStr(Unescaped::new_unchecked(", ")),
            ViewPart::UnescapedString(Unescaped::new_unchecked(
                serde_json::to_string(&self.path).unwrap(),
            )),
            ViewPart::UnescapedStaticStr(Unescaped::new_unchecked(") -->")),
            self.placeholder.into_inner(),
            ViewPart::UnescapedStaticStr(Unescaped::new_unchecked("<!-- ::topcoat::scope::end(")),
            ViewPart::UnescapedString(Unescaped::new_unchecked(
                serde_json::to_string(&self.id).unwrap(),
            )),
            ViewPart::UnescapedStaticStr(Unescaped::new_unchecked(") -->")),
        ]
        .into_iter()
    }
}
