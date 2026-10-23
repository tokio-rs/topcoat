// Show router links as plain text when the router feature is disabled.
#![cfg_attr(not(feature = "router"), allow(rustdoc::broken_intra_doc_links))]

use topcoat_core::{
    context::{Cx, try_app_context, try_request_context},
    error::Result,
};
use topcoat_view::{AttributeValueViewParts, Attributes, Child, View};
use topcoat_view_macro::{component, view};

/// Enables runtime navigation on an anchor and sets its prefetch mode.
const LINK_ATTRIBUTE: &str = "data-topcoat-link";

/// Declares the View Transition types a link's navigation runs inside.
///
/// Prefer the `vt` argument on [`link`]. Use this constant when writing
/// your own `<a>` markup instead of calling [`link`].
pub const VT_ATTRIBUTE: &str = "data-topcoat-vt-types";

/// Marks the element that morphs into its counterpart on the destination.
///
/// The value becomes the element's [`view-transition-name`][name] when the
/// link is followed, pairing it with the destination element that declares
/// the same name in its style. After the swap, the destination's markup
/// determines the styles the element keeps.
///
/// [name]: https://developer.mozilla.org/docs/Web/CSS/view-transition-name
pub const VT_SHARED_ATTRIBUTE: &str = "data-topcoat-vt-shared";

/// Controls when the browser loads a linked page before the user follows it.
///
/// Loading a page early, or prefetching, can make navigation faster. The
/// runtime may skip it, for example if the browser is set to save data.
/// Prefetching renders the page on the server even if the user never opens
/// it, so rendering a page must be safe in that case.
///
/// Set the `prefetch` argument on [`link`] to choose when to load that page.
/// Otherwise, the link uses the default returned by [`prefetch_mode`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrefetchMode {
    /// Loads the page only after the user follows the link.
    Never,
    /// Loads the page after the user hovers over the link briefly or as
    /// soon as the link gets focus. This is the default.
    #[default]
    Intent,
    /// Loads the page when the link comes into view.
    Viewport,
}

impl PrefetchMode {
    /// Returns the mode's name for use in the HTML attribute.
    const fn as_str(self) -> &'static str {
        match self {
            Self::Never => "never",
            Self::Intent => "intent",
            Self::Viewport => "viewport",
        }
    }
}

/// Returns the default prefetch mode for links in this context.
///
/// Checks these sources in order and uses the first value it finds:
///
/// 1. A `PrefetchMode` in the request context.
/// 2. A `PrefetchMode` in the app context.
/// 3. [`PrefetchMode::Intent`].
///
/// Set the default for the app with
/// [`RouterBuilderRuntimeExt::prefetch`](crate::RouterBuilderRuntimeExt::prefetch),
/// or use [`Cx::with`] to set a `PrefetchMode` for links rendered in a
/// particular scope.
#[must_use]
pub fn prefetch_mode(cx: &Cx) -> PrefetchMode {
    try_request_context::<PrefetchMode>(cx)
        .or_else(|| try_app_context::<PrefetchMode>(cx))
        .copied()
        .unwrap_or_default()
}

/// Creates attributes that let an `<a>` element use runtime navigation.
///
/// Spread the returned attributes into your own `<a>` element to give it
/// the same behavior as [`link`]. They set its `href` and enable runtime
/// navigation with the chosen `prefetch` mode. Use an
/// [`href!`](https://docs.rs/topcoat/latest/topcoat/router/macro.href.html)
/// value for a page in your app. Call [`prefetch_mode`] to get the default
/// mode for the current context.
///
/// Without JavaScript, the element still works as a regular link. The
/// browser also handles clicks with modifier keys, links to other windows
/// or frames, downloads, and links to other sites as usual.
#[must_use]
pub fn link_attrs(
    cx: &Cx,
    href: impl AttributeValueViewParts,
    prefetch: PrefetchMode,
) -> Attributes {
    let mut attrs = Attributes::with_capacity(2);
    attrs.insert(cx, "href", href);
    attrs.insert(cx, LINK_ATTRIBUTE, prefetch.as_str());
    attrs
}

#[doc = include_str!("../docs/link.md")]
#[component]
pub async fn link<H>(
    cx: &Cx,
    /// The URL to open. Use `href!` for pages in your app.
    href: H,
    /// When to load the page before the user follows the link. Uses
    /// [`prefetch_mode`] if omitted.
    #[into]
    #[default]
    prefetch: Option<PrefetchMode>,
    /// Space-separated View Transition types the navigation runs inside.
    /// Omitted by default: the navigation swaps without a transition.
    ///
    /// To morph an element into its counterpart on the destination, mark it
    /// with [`VT_SHARED_ATTRIBUTE`].
    #[into]
    #[default]
    vt: Option<&str>,
    /// Additional HTML attributes to put on the link.
    #[default]
    mut attrs: Attributes,
    /// The text or other content inside the link.
    #[default]
    child: Child<'_>,
) -> Result<impl View>
where
    H: AttributeValueViewParts + Send,
{
    let prefetch = prefetch.unwrap_or_else(|| prefetch_mode(cx));
    attrs.extend(link_attrs(cx, href, prefetch));
    if let Some(vt) = vt {
        attrs.insert(cx, VT_ATTRIBUTE, vt);
    }
    Ok(view! { <a (attrs)>(child)</a> })
}

#[cfg(test)]
mod tests {
    use std::{
        pin::pin,
        task::{Context, Poll, Waker},
    };

    use topcoat::view::{ViewExt, attributes, view};
    use topcoat_core::context::CxTestBuilder;

    use super::*;

    /// Runs a future to completion by polling it repeatedly without a pause.
    fn block_on<F: Future>(future: F) -> F::Output {
        let mut future = pin!(future);
        let mut cx = Context::from_waker(Waker::noop());
        loop {
            if let Poll::Ready(output) = future.as_mut().poll(&mut cx) {
                return output;
            }
        }
    }

    fn render(view: impl View) -> String {
        block_on(view.single()).unwrap().render(&Cx::default())
    }

    #[test]
    fn link_attrs_mark_the_anchor_with_its_destination_and_mode() {
        let cx = &Cx::default();
        let attrs = link_attrs(cx, "/products", PrefetchMode::Viewport);
        let html = render(view! { cx => <a (attrs)>"Products"</a> });

        assert!(html.contains(r#"href="/products""#), "{html}");
        assert!(html.contains(r#"data-topcoat-link="viewport""#), "{html}");
    }

    #[test]
    fn every_mode_renders_differently() {
        let cx = &Cx::default();
        let mut seen = Vec::new();
        for prefetch in [
            PrefetchMode::Never,
            PrefetchMode::Intent,
            PrefetchMode::Viewport,
        ] {
            let attrs = link_attrs(cx, "/", prefetch);
            let html = render(view! { cx => <a (attrs)></a> });
            assert!(!seen.contains(&html), "{html}");
            seen.push(html);
        }
    }

    #[test]
    fn link_renders_an_anchor_with_forwarded_attributes() {
        let cx = &Cx::default();
        let attrs = attributes! { cx => class="nav" aria-current="page" };
        let html = render(view! { cx => link(href: "/account", attrs: attrs, "Account") });

        assert!(html.starts_with("<a "), "{html}");
        assert!(html.contains(r#"href="/account""#), "{html}");
        assert!(html.contains(r#"class="nav""#), "{html}");
        assert!(html.contains(r#"aria-current="page""#), "{html}");
        assert!(html.contains(">Account</a>"), "{html}");
        // The default mode loads the page on hover or focus.
        assert!(html.contains(r#"data-topcoat-link="intent""#), "{html}");
    }

    #[test]
    fn explicit_props_take_precedence_over_forwarded_attributes() {
        let cx = &Cx::default();
        let attrs = attributes! { cx => href="/elsewhere" data-topcoat-link="viewport" };
        let html = render(view! {
            cx =>
            link(href: "/account", prefetch: PrefetchMode::Never, attrs: attrs)
        });

        assert!(html.contains(r#"href="/account""#), "{html}");
        assert!(!html.contains("/elsewhere"), "{html}");
        assert!(!html.contains("viewport"), "{html}");
    }

    #[test]
    fn prefetch_mode_prefers_the_request_context_over_the_app_context() {
        let app = CxTestBuilder::new()
            .app_context(PrefetchMode::Viewport)
            .build();
        assert_eq!(prefetch_mode(&app), PrefetchMode::Viewport);

        let scoped = app.with(PrefetchMode::Never);
        assert_eq!(prefetch_mode(&scoped), PrefetchMode::Never);
    }

    #[test]
    fn link_uses_the_context_mode_unless_given_one() {
        let cx = &CxTestBuilder::new()
            .app_context(PrefetchMode::Viewport)
            .build();
        let html = render(view! { cx => link(href: "/", "Home") });
        assert!(html.contains(r#"data-topcoat-link="viewport""#), "{html}");

        let explicit = render(view! {
            cx =>
            link(href: "/", prefetch: PrefetchMode::Never, "Home")
        });
        assert!(
            explicit.contains(r#"data-topcoat-link="never""#),
            "{explicit}"
        );
    }

    #[test]
    fn link_renders_its_transition_types() {
        let cx = &Cx::default();
        let html = render(view! {
            cx =>
            link(href: "/articles/7", vt: "detail", "Read more")
        });

        assert!(html.contains(r#"data-topcoat-vt-types="detail""#), "{html}");
    }

    #[test]
    fn a_link_declares_no_transition_by_default() {
        let cx = &Cx::default();
        let html = render(view! { cx => link(href: "/", "Home") });

        assert!(!html.contains("view-transition"), "{html}");
    }

    #[test]
    fn link_carries_no_shared_name_of_its_own() {
        let cx = &Cx::default();
        // The shared name belongs on the element that carries it, not on the
        // anchor, so `link` never emits the attribute itself.
        let html = render(view! {
            cx =>
            link(href: "/articles/7", vt: "detail", "Read more")
        });

        assert!(html.contains(r#"data-topcoat-vt-types="detail""#), "{html}");
        assert!(!html.contains(VT_SHARED_ATTRIBUTE), "{html}");
    }

    #[test]
    fn the_shared_marker_is_usable_from_an_application() {
        let cx = &Cx::default();
        // What an application writes on the element inside the link.
        let attrs = attributes! { cx => data-topcoat-vt-shared=("article-7") };
        let html = render(view! { cx =>
            link(
                href: "/articles/7",
                vt: "detail",
                <img (attrs) src="/shoes.jpg" alt="Shoes" />
            )
        });

        assert!(
            html.contains(r#"data-topcoat-vt-shared="article-7""#),
            "{html}"
        );
        assert!(
            VT_SHARED_ATTRIBUTE.starts_with("data-topcoat-"),
            "{VT_SHARED_ATTRIBUTE}"
        );
    }

    #[test]
    fn transition_arguments_override_matching_forwarded_attributes() {
        let cx = &Cx::default();
        let attrs = attributes! { cx => data-topcoat-vt-types="stale" };
        let html = render(view! {
            cx =>
            link(href: "/articles/7", vt: "detail", attrs: attrs, "Read more")
        });

        assert!(html.contains(r#"data-topcoat-vt-types="detail""#), "{html}");
        assert!(!html.contains("stale"), "{html}");
    }
}
