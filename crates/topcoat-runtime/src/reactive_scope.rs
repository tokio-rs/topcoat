use serde::Serialize;
use topcoat_core::context::Cx;
use topcoat_view::{NodeViewParts, PartsWriter, View, ViewPart};
use uuid::Uuid;

use crate::{SHARD_ROUTE_PREFIX, ShardId};

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
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        let shard_id = self.shard_id.as_str();

        // <!-- ::topcoat::scope::start("<id>", "<path>", ["<js>", ...]) -->
        //
        // Each parameter's JavaScript source is wrapped in a quoted string.
        // The source parts are sealed with the comment context, so any `"`
        // inside the source renders as `&quot;` and the quotes stay
        // unambiguous delimiters on the client.
        parts.push_str_unescaped("<!-- ::topcoat::scope::start(");
        parts.push_str_unescaped(serde_json::to_string(&self.id).unwrap());
        parts.push_str_unescaped(", ");
        parts.push_str_unescaped(
            serde_json::to_string(&format!("{SHARD_ROUTE_PREFIX}/{shard_id}")).unwrap(),
        );
        parts.push_str_unescaped(", [");
        let last = self.exprs.len().saturating_sub(1);
        for (index, expr) in self.exprs.into_iter().enumerate() {
            parts.push_str_unescaped("\"");
            parts.push_part(expr);
            parts.push_str_unescaped("\"");
            if index != last {
                parts.push_str_unescaped(", ");
            }
        }
        parts.push_str_unescaped("]) -->");
        self.placeholder.into_view_parts(cx, parts);
        parts.push_str_unescaped("<!-- ::topcoat::scope::end(");
        parts.push_str_unescaped(serde_json::to_string(&self.id).unwrap());
        parts.push_str_unescaped(") -->");
    }
}
