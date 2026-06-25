use serde::Serialize;
use topcoat_view::runtime::{NodeViewParts, Unescaped, View, ViewPart, ViewParts};
use uuid::Uuid;

use crate::runtime::{SHARD_ROUTE_PREFIX, ShardId};

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
    shard_id: ShardId,
    exprs: Vec<ViewPart>,
    placeholder: View,
}

impl ReactiveScope {
    #[inline]
    #[must_use]
    pub fn new(shard_id: ShardId, exprs: Vec<ViewPart>, placeholder: View) -> Self {
        Self {
            id: ReactiveScopeId::new(),
            shard_id,
            exprs,
            placeholder,
        }
    }
}

impl NodeViewParts for ReactiveScope {
    fn into_view_parts(self, parts: &mut ViewParts) {
        let shard_id = self.shard_id.as_str();

        // <!-- ::topcoat::scope::start("<id>", "<path>", ["<js>", ...]) -->
        //
        // Each parameter's JavaScript source is wrapped in a quoted string.
        // Because the parts are HTML-escaped, any `"` inside the source becomes
        // `&quot;`, so the quotes stay unambiguous delimiters on the client.
        parts.push(Unescaped::new_unchecked("<!-- ::topcoat::scope::start("));
        parts.push(Unescaped::new_unchecked(
            serde_json::to_string(&self.id).unwrap(),
        ));
        parts.push(Unescaped::new_unchecked(", "));
        parts.push(Unescaped::new_unchecked(
            serde_json::to_string(&format!("{SHARD_ROUTE_PREFIX}/{shard_id}")).unwrap(),
        ));
        parts.push(Unescaped::new_unchecked(", ["));
        let last = self.exprs.len().saturating_sub(1);
        for (index, expr) in self.exprs.into_iter().enumerate() {
            parts.push(Unescaped::new_unchecked("\""));
            parts.push(expr);
            parts.push(Unescaped::new_unchecked("\""));
            if index != last {
                parts.push(Unescaped::new_unchecked(", "));
            }
        }
        parts.push(Unescaped::new_unchecked("]) -->"));
        parts.push(self.placeholder);
        parts.push(Unescaped::new_unchecked("<!-- ::topcoat::scope::end("));
        parts.push(Unescaped::new_unchecked(
            serde_json::to_string(&self.id).unwrap(),
        ));
        parts.push(Unescaped::new_unchecked(") -->"));
    }
}
