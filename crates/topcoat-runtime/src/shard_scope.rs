use std::{
    pin::Pin,
    task::{Context, Poll, ready},
};

use pin_project_lite::pin_project;
use topcoat_core::{context::Cx, error::Result, identity::Identity};
use topcoat_view::{
    NodeViewParts, PartsWriter, View, ViewFirst, ViewSwap, internal::Builder,
};

use crate::Js;

pin_project! {
    /// A shard invocation's content between the markers the browser finds
    /// it by.
    ///
    /// The markers surround the shard body's first content. Later updates
    /// from a live body pass through unchanged, so a live shard keeps
    /// streaming like any other live content.
    pub struct ShardScope<'cx, V> {
        cx: &'cx Cx,
        // The identity of the shard invocation, which names the scope in
        // its markers and which the browser sends back with a re-render
        // request so the shard body derives the same identities as the
        // inline render.
        identity: Identity,
        // The request URL, with route groups removed and `/` for the root.
        url: &'static str,
        exprs: Vec<Js>,
        #[pin]
        view: V,
    }
}

impl<'cx, V> ShardScope<'cx, V> {
    #[inline]
    #[must_use]
    pub fn new(
        cx: &'cx Cx,
        identity: Identity,
        url: &'static str,
        exprs: Vec<Js>,
        view: V,
    ) -> Self {
        Self {
            cx,
            identity,
            url,
            exprs,
            view,
        }
    }
}

impl<V: View> View for ShardScope<'_, V> {
    fn poll_first(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<ViewFirst>> {
        let this = self.project();
        let first = ready!(this.view.poll_first(cx))?;
        let identity = serde_json::to_string(&this.identity.to_string()).unwrap();
        let content = Builder::block(this.cx, |builder| {
            builder.node(StartMarker {
                url: this.url,
                identity: &identity,
                exprs: this.exprs,
            });
            builder.view(first.content);
            builder.node(EndMarker { identity });
        });
        Poll::Ready(Ok(ViewFirst {
            content,
            live: first.live,
        }))
    }

    fn poll_swap(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Option<ViewSwap>>> {
        self.project().view.poll_swap(cx)
    }
}

/// The comment opening a shard's content.
///
/// The browser runtime posts re-render requests to the URL. The identity
/// is stable across renders, so a re-render of the enclosing content
/// produces the same markers and the browser can keep them. Each
/// parameter's JavaScript source is wrapped in a quoted string. The source
/// parts are sealed with the comment context, so any `"` inside the source
/// renders as `&quot;` and the quotes stay unambiguous delimiters on the
/// client.
struct StartMarker<'a> {
    url: &'static str,
    /// The identity as a JSON string, quotes included.
    identity: &'a str,
    exprs: &'a [Js],
}

impl NodeViewParts for StartMarker<'_> {
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        // <!-- ::topcoat::shard::start("<url>", "<identity>", ["<js>", ...]) -->
        parts.push_comment(|comment| {
            comment
                .push_promoted_str_unescaped(&"::topcoat::shard::start(")
                .push_string_unescaped(serde_json::to_string(self.url).unwrap())
                .push_promoted_str_unescaped(&", ")
                .push_string_unescaped(self.identity.to_owned())
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
    }
}

/// The comment closing a shard's content.
struct EndMarker {
    /// The identity as a JSON string, quotes included.
    identity: String,
}

impl NodeViewParts for EndMarker {
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        // <!-- ::topcoat::shard::end("<identity>") -->
        parts.push_comment(|comment| {
            comment
                .push_promoted_str_unescaped(&"::topcoat::shard::end(")
                .push_string_unescaped(self.identity)
                .push_promoted_str_unescaped(&")");
        });
    }
}
