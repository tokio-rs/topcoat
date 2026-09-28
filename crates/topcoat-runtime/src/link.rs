// Links to the router extension become plain text without the router.
#![cfg_attr(not(feature = "router"), allow(rustdoc::broken_intra_doc_links))]

use topcoat_core::{
    context::{Cx, try_app_context, try_request_context},
    error::Result,
};
use topcoat_view::{AttributeValueViewParts, Attributes, Child, View};
use topcoat_view_macro::{component, view};

/// The attribute that opts an anchor into runtime navigation. Its value is
/// the prefetch mode.
const LINK_ATTRIBUTE: &str = "data-topcoat-link";

/// When the browser runtime fetches a link's destination ahead of a click.
///
/// Prefetching is a best-effort optimization. The runtime may skip it, for
/// example when the browser asks to save data. A prefetch renders the
/// destination on the server, so pages must be safe to render
/// speculatively.
///
/// Pass `prefetch` to [`link`] to choose the mode for one link. Links
/// without one use [`prefetch_mode`], which reads a default from the
/// context.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrefetchMode {
    /// Fetches the destination only when the link is followed.
    Never,
    /// Fetches the destination after the pointer rests on the link, or as
    /// soon as the link receives focus. This is the default.
    #[default]
    Intent,
    /// Fetches the destination once the link becomes visible.
    Viewport,
}

impl PrefetchMode {
    /// Returns the attribute value the browser runtime reads.
    const fn as_str(self) -> &'static str {
        match self {
            Self::Never => "never",
            Self::Intent => "intent",
            Self::Viewport => "viewport",
        }
    }
}

/// Returns the prefetch mode for links that do not choose one.
///
/// Returns the first available value in this order:
///
/// 1. A `PrefetchMode` in the request context.
/// 2. A `PrefetchMode` in the app context.
/// 3. [`PrefetchMode::Intent`].
///
/// Set the app default with
/// [`RouterBuilderRuntimeExt::prefetch`](crate::RouterBuilderRuntimeExt::prefetch),
/// or add a `PrefetchMode` to a scope with [`Cx::with`] to override it for
/// the links rendered there.
#[must_use]
pub fn prefetch_mode(cx: &Cx) -> PrefetchMode {
    try_request_context::<PrefetchMode>(cx)
        .or_else(|| try_app_context::<PrefetchMode>(cx))
        .copied()
        .unwrap_or_default()
}

/// Builds the attributes of an anchor that navigates with the browser
/// runtime.
///
/// The result holds the `href` for `href` and the attribute that enables
/// runtime navigation with the `prefetch` mode. Spread it into an `<a>`
/// element to give custom markup the behavior of [`link`]. Pass an
/// [`href!`](https://docs.rs/topcoat/latest/topcoat/router/macro.href.html)
/// value for internal destinations, and [`prefetch_mode`] to use the
/// context's default mode.
///
/// The anchor works as a normal link without JavaScript, and the browser
/// keeps handling modified clicks, other targets, downloads, and external
/// destinations.
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

/// An anchor that navigates with the browser runtime.
///
/// Following the link renders the destination on the server and updates
/// the document in place, keeping the signals the destination declares
/// again. The anchor works as a normal link without JavaScript, and the
/// browser keeps handling modified clicks, other targets, downloads, and
/// external destinations.
///
/// When `prefetch` is omitted, [`prefetch_mode`] selects the mode from the
/// context. `attrs` adds attributes to the `<a>` element, such as classes
/// or `aria-*` attributes; `href` and `prefetch` take precedence over the
/// attributes they produce. Use [`link_attrs`] to give the same behavior to
/// custom markup.
///
/// ```
/// use topcoat::view::view;
/// use topcoat_runtime::{PrefetchMode, link};
///
/// # #[topcoat::view::component]
/// # async fn example() -> topcoat::Result<impl topcoat::view::View> {
/// Ok(view! {
///     link(href: "/products", "Products")
///     link(href: "/reports", prefetch: PrefetchMode::Never, "Reports")
/// })
/// # }
/// ```
#[component]
pub async fn link<H>(
    cx: &Cx,
    /// The destination. Use an `href!` value for internal pages.
    href: H,
    /// When to fetch the destination ahead of a click. Defaults to the
    /// mode selected by [`prefetch_mode`].
    #[into]
    #[default]
    prefetch: Option<PrefetchMode>,
    /// Extra attributes for the `<a>` element.
    #[default]
    mut attrs: Attributes,
    /// The link's content.
    #[default]
    child: Child<'_>,
) -> Result<impl View>
where
    H: AttributeValueViewParts + Send,
{
    let prefetch = prefetch.unwrap_or_else(|| prefetch_mode(cx));
    attrs.extend(link_attrs(cx, href, prefetch));
    Ok(view! {
        <a (attrs)>(child)</a>
    })
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

    /// Polls a future until it completes, without waiting between polls.
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
        // Links prefetch on intent unless told otherwise.
        assert!(html.contains(r#"data-topcoat-link="intent""#), "{html}");
    }

    #[test]
    fn explicit_props_take_precedence_over_forwarded_attributes() {
        let cx = &Cx::default();
        let attrs = attributes! { cx => href="/elsewhere" data-topcoat-link="viewport" };
        let html = render(view! { cx =>
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

        let explicit = render(view! { cx =>
            link(href: "/", prefetch: PrefetchMode::Never, "Home")
        });
        assert!(
            explicit.contains(r#"data-topcoat-link="never""#),
            "{explicit}"
        );
    }
}
