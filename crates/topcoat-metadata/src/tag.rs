//! Components that each render one kind of metadata tag.
//!
//! [`tags`](crate::tags) renders all of them. Use these instead to pick the
//! tags to render, for example to write one of them yourself. Read the
//! metadata once with [`metadata`](crate::metadata) and pass it to each
//! component.

use topcoat_core::{context::Cx, error::Result, url_form::UrlForm};
use topcoat_view::View;
use topcoat_view_macro::{component, view};

use crate::PageMetadata;

/// Renders `<meta charset="utf-8">`.
///
/// Browsers only look for it in the first 1024 bytes of a document, so
/// render it at the start of `<head>`.
#[component]
pub async fn charset() -> Result<impl View> {
    Ok(view! { <meta charset="utf-8"> })
}

/// Renders the [viewport](crate::Metadata::viewport) of a page.
#[component]
pub async fn viewport(
    /// The metadata of the page.
    metadata: &PageMetadata,
) -> Result<impl View> {
    Ok(view! {
        if let Some(viewport) = metadata.viewport() {
            <meta name="viewport" content=(viewport)>
        }
    })
}

/// Renders the [title](crate::Metadata::title) of a page.
#[component]
pub async fn title(
    /// The metadata of the page.
    metadata: &PageMetadata,
) -> Result<impl View> {
    Ok(view! {
        if let Some(title) = metadata.title() {
            <title>(title)</title>
        }
    })
}

/// Renders the [description](crate::Metadata::description) of a page.
#[component]
pub async fn description(
    /// The metadata of the page.
    metadata: &PageMetadata,
) -> Result<impl View> {
    Ok(view! {
        if let Some(description) = metadata.description() {
            <meta name="description" content=(description)>
        }
    })
}

/// Renders the [canonical URL](crate::Metadata::canonical) of a page, always
/// in the absolute form.
#[component]
pub async fn canonical(
    cx: &Cx,
    /// The metadata of the page.
    metadata: &PageMetadata,
) -> Result<impl View> {
    let absolute = cx.with(UrlForm::Absolute);
    Ok(view! {
        if let Some(canonical) = metadata.canonical() {
            (view! { absolute => <link rel="canonical" href=(canonical)> })
        }
    })
}

/// Renders the [robots directives](crate::Robots) of a page.
///
/// Renders nothing when they allow everything, which is what search engines
/// assume without the tag.
#[component]
pub async fn robots(
    /// The metadata of the page.
    metadata: &PageMetadata,
) -> Result<impl View> {
    let robots = metadata.robots();
    let content = match (robots.index(), robots.follow()) {
        (true, true) => None,
        (false, true) => Some("noindex"),
        (true, false) => Some("nofollow"),
        (false, false) => Some("noindex, nofollow"),
    };
    Ok(view! {
        if let Some(content) = content {
            <meta name="robots" content=(content)>
        }
    })
}

/// Renders the [theme color](crate::Metadata::theme_color) of a page.
#[component]
pub async fn theme_color(
    /// The metadata of the page.
    metadata: &PageMetadata,
) -> Result<impl View> {
    Ok(view! {
        if let Some(color) = metadata.theme_color() {
            <meta name="theme-color" content=(color)>
        }
    })
}

/// Renders the [Open Graph metadata](crate::OpenGraph) of a page, which
/// social networks use for link previews. URLs are always in the absolute
/// form.
#[component]
pub async fn open_graph(
    cx: &Cx,
    /// The metadata of the page.
    metadata: &PageMetadata,
) -> Result<impl View> {
    let og = metadata.open_graph();
    let absolute = cx.with(UrlForm::Absolute);
    Ok(view! {
        <meta property="og:type" content=(og.kind().as_str())>
        if let Some(og_title) = og.title() {
            <meta property="og:title" content=(og_title)>
        }
        if let Some(og_description) = og.description() {
            <meta property="og:description" content=(og_description)>
        }
        if let Some(site_name) = og.site_name() {
            <meta property="og:site_name" content=(site_name)>
        }
        (view! {
            absolute =>
            if let Some(url) = og.url() {
                <meta property="og:url" content=(url)>
            }
            for image in og.images() {
                <meta property="og:image" content=(&image.url)>
                if let Some(width) = image.width {
                    <meta property="og:image:width" content=(width)>
                }
                if let Some(height) = image.height {
                    <meta property="og:image:height" content=(height)>
                }
                if let Some(alt) = &image.alt {
                    <meta property="og:image:alt" content=(alt)>
                }
            }
        })
    })
}

/// Renders the [X metadata](crate::Twitter) of a page, which X uses for link
/// previews together with the [Open Graph metadata](open_graph).
#[component]
pub async fn twitter(
    /// The metadata of the page.
    metadata: &PageMetadata,
) -> Result<impl View> {
    let x = metadata.twitter();
    Ok(view! {
        <meta name="twitter:card" content=(x.card().as_str())>
        if let Some(site) = x.site() {
            <meta name="twitter:site" content=(site)>
        }
        if let Some(creator) = x.creator() {
            <meta name="twitter:creator" content=(creator)>
        }
    })
}

#[cfg(test)]
mod tests {
    use topcoat_core::base_url::BaseUrl;
    use topcoat_view::ViewExt;

    use super::*;
    use crate::{Metadata, metadata};

    #[tokio::test]
    async fn renders_the_canonical_url_in_the_absolute_form() {
        let cx = &Cx::new()
            .with(BaseUrl::new("https://example.com").unwrap())
            .with(Metadata::new().canonical("/about"));
        let page = metadata(cx).await.unwrap();

        let html = view! { cx => canonical(metadata: page) }
            .single()
            .await
            .unwrap()
            .render(cx);

        assert!(
            html.contains(r#"href="https://example.com/about""#),
            "{html}"
        );
    }

    #[tokio::test]
    async fn renders_open_graph_urls_in_the_absolute_form() {
        let cx = &Cx::new()
            .with(BaseUrl::new("https://example.com").unwrap())
            .with(
                Metadata::new()
                    .canonical("/teapot")
                    .open_graph(|og| og.image(crate::Image::new("/teapot.png").size(1200, 630))),
            );
        let page = metadata(cx).await.unwrap();

        let html = view! { cx => open_graph(metadata: page) }
            .single()
            .await
            .unwrap()
            .render(cx);

        assert!(
            html.contains(r#"property="og:url" content="https://example.com/teapot""#),
            "{html}"
        );
        assert!(
            html.contains(r#"property="og:image" content="https://example.com/teapot.png""#),
            "{html}"
        );
        assert!(
            html.contains(r#"property="og:image:width" content="1200""#),
            "{html}"
        );
    }

    #[tokio::test]
    async fn renders_robots_only_when_a_directive_is_denied() {
        let allowed = &Cx::new();
        let page = metadata(allowed).await.unwrap();
        let html = view! { allowed => robots(metadata: page) }
            .single()
            .await
            .unwrap()
            .render(allowed);
        assert!(!html.contains("robots"), "{html}");

        let denied = &Cx::new().with(Metadata::new().robots(|r| r.index(false)));
        let page = metadata(denied).await.unwrap();
        let html = view! { denied => robots(metadata: page) }
            .single()
            .await
            .unwrap()
            .render(denied);
        assert!(html.contains("noindex"), "{html}");
    }
}
