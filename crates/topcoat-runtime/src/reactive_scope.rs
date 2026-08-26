use serde::Serialize;
use topcoat_core::context::Cx;
use topcoat_view::{NodeViewParts, PartsWriter, ViewHandle};
use uuid::Uuid;

use crate::{Js, SHARD_ROUTE_PREFIX, ShardId};

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
    exprs: Vec<Js>,
    placeholder: ViewHandle,
}

impl ReactiveScope {
    #[inline]
    #[must_use]
    pub fn new(shard_id: ShardId, exprs: Vec<Js>, placeholder: ViewHandle) -> Self {
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
        parts.push_comment(|comment| {
            comment
                .push_promoted_str_unescaped(&"::topcoat::scope::start(")
                .push_string_unescaped(serde_json::to_string(&self.id).unwrap())
                .push_promoted_str_unescaped(&", ")
                .push_string_unescaped(
                    serde_json::to_string(&format!("{SHARD_ROUTE_PREFIX}/{shard_id}")).unwrap(),
                )
                .push_promoted_str_unescaped(&", [");
            let last = self.exprs.len().saturating_sub(1);
            for (index, expr) in self.exprs.iter().enumerate() {
                comment.push_promoted_str_unescaped(&"\"");
                expr.write(comment);
                comment.push_promoted_str_unescaped(&"\"");
                if index != last {
                    comment.push_promoted_str_unescaped(&", ");
                }
            }
            comment.push_promoted_str_unescaped(&"])");
        });
        self.placeholder.into_view_parts(cx, parts);
        parts.push_comment(|comment| {
            comment
                .push_promoted_str_unescaped(&"::topcoat::scope::end(")
                .push_string_unescaped(serde_json::to_string(&self.id).unwrap())
                .push_promoted_str_unescaped(&")");
        });
    }
}
