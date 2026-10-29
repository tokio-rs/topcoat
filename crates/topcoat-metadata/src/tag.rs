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
