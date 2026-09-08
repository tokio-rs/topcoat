use serde::Serialize;
use topcoat_core::context::Cx;
use topcoat_view::{NodeViewParts, PartsWriter, ViewHandle, identity::Identity};
use uuid::Uuid;

use crate::{Js, ShardId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct ShardScopeId(Uuid);

impl ShardScopeId {
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ShardScopeId {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

pub struct ShardScope {
    id: ShardScopeId,
    /// The identity of the shard invocation, which the browser sends back
    /// with a re-render request so the shard body derives the same
    /// identities as the inline render.
    identity: Identity,
    shard_id: ShardId,
    exprs: Vec<Js>,
    placeholder: ViewHandle,
}

impl ShardScope {
    #[inline]
    #[must_use]
    pub fn new(
        identity: Identity,
        shard_id: ShardId,
        exprs: Vec<Js>,
        placeholder: ViewHandle,
    ) -> Self {
        Self {
            id: ShardScopeId::new(),
            identity,
            shard_id,
            exprs,
            placeholder,
        }
    }
}

impl NodeViewParts for ShardScope {
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        let shard_id = self.shard_id.as_str();

        // <!-- ::topcoat::shard::start("<id>", "<shard id>", "<identity>", ["<js>", ...]) -->
        //
        // The browser runtime derives the shard's route from its id. Each
        // parameter's JavaScript source is wrapped in a quoted string. The
        // source parts are sealed with the comment context, so any `"`
        // inside the source renders as `&quot;` and the quotes stay
        // unambiguous delimiters on the client.
        parts.push_comment(|comment| {
            comment
                .push_promoted_str_unescaped(&"::topcoat::shard::start(")
                .push_string_unescaped(serde_json::to_string(&self.id).unwrap())
                .push_promoted_str_unescaped(&", ")
                .push_string_unescaped(serde_json::to_string(shard_id).unwrap())
                .push_promoted_str_unescaped(&", ")
                .push_string_unescaped(serde_json::to_string(&self.identity.to_string()).unwrap())
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
                .push_promoted_str_unescaped(&"::topcoat::shard::end(")
                .push_string_unescaped(serde_json::to_string(&self.id).unwrap())
                .push_promoted_str_unescaped(&")");
        });
    }
}
