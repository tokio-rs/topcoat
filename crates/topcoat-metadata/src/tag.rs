//! Components for rendering individual kinds of metadata tags.
//!
//! Use [`tags`](crate::tags) to render the full set, or call these components
//! individually to choose the tags you need. This also lets you replace a
//! tag with your own markup. Read [`metadata`](crate::metadata) once and pass
//! the result to each component.

use topcoat_core::{context::Cx, error::Result, url_form::UrlForm};
use topcoat_view::View;
use topcoat_view_macro::{component, view};

use crate::PageMetadata;

/// Declares UTF-8 with `<meta charset="utf-8">`.
///
/// Put this tag first in `<head>`. Browsers look for the charset declaration
/// within the document's first 1024 bytes.
#[component]
pub async fn charset() -> Result<impl View> {
    Ok(view! { <meta charset="utf-8"> })
}

/// Writes the page's [viewport tag](crate::Metadata::viewport), if present.
#[component]
pub async fn viewport(
    /// The combined metadata to render.
    metadata: &PageMetadata,
) -> Result<impl View> {
    Ok(view! {
        if let Some(viewport) = metadata.viewport() {
            <meta name="viewport" content=(viewport)>
        }
    })
}

/// Writes a `<title>` tag for the page's [title](crate::Metadata::title), if set.
#[component]
pub async fn title(
    /// The combined metadata to render.
    metadata: &PageMetadata,
) -> Result<impl View> {
    Ok(view! {
        if let Some(title) = metadata.title() {
            <title>(title)</title>
        }
    })
}

/// Writes a meta tag for the page's [description](crate::Metadata::description), if set.
#[component]
pub async fn description(
    /// The combined metadata to render.
    metadata: &PageMetadata,
) -> Result<impl View> {
    Ok(view! {
        if let Some(description) = metadata.description() {
            <meta name="description" content=(description)>
        }
    })
}

/// Writes a canonical link tag if the page has a
/// [canonical URL](crate::Metadata::canonical). Requests the absolute URL form.
#[component]
pub async fn canonical(
    cx: &Cx,
    /// The combined metadata to render.
    metadata: &PageMetadata,
) -> Result<impl View> {
    let absolute = cx.with(UrlForm::Absolute);
    Ok(view! {
        if let Some(canonical) = metadata.canonical() {
            (view! { absolute => <link rel="canonical" href=(canonical)> })
        }
    })
}

/// Writes the page's [search engine instructions](crate::Robots) in a robots tag.
///
/// Omits the tag when indexing and following links are both allowed, since
/// those are the defaults without a tag.
#[component]
pub async fn robots(
    /// The combined metadata to render.
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

/// Writes a meta tag for the page's [theme color](crate::Metadata::theme_color), if set.
#[component]
pub async fn theme_color(
    /// The combined metadata to render.
    metadata: &PageMetadata,
) -> Result<impl View> {
    Ok(view! {
        if let Some(color) = metadata.theme_color() {
            <meta name="theme-color" content=(color)>
        }
    })
}

/// Writes [Open Graph tags](crate::OpenGraph) for social link previews.
/// Requests the absolute form for page and image URLs.
#[component]
pub async fn open_graph(
    cx: &Cx,
    /// The combined metadata to render.
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

/// Writes the page's [X preview tags](crate::Twitter). X also uses the
/// [Open Graph tags](open_graph) for the preview's content.
#[component]
pub async fn twitter(
    /// The combined metadata to render.
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
