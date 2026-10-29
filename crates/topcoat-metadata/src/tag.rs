//! Components that each render one kind of metadata tag.
//!
//! [`tags`](crate::tags) renders all of them. Use these instead to pick the
//! tags to render, for example to write one of them yourself.

use topcoat_core::{context::Cx, error::Result, url_form::UrlForm};
use topcoat_view::View;
use topcoat_view_macro::{component, view};

use crate::metadata;

/// Renders `<meta charset="utf-8">`.
///
/// Browsers only look for it in the first 1024 bytes of a document, so
/// render it at the start of `<head>`.
#[component]
pub async fn charset() -> Result<impl View> {
    Ok(view! { <meta charset="utf-8"> })
}

/// Renders the [viewport](crate::Metadata::viewport) of the current page.
#[component]
pub async fn viewport(cx: &Cx) -> Result<impl View> {
    let metadata = metadata(cx).await?;
    Ok(view! {
        if let Some(viewport) = metadata.viewport() {
            <meta name="viewport" content=(viewport)>
        }
    })
}

/// Renders the [title](crate::Metadata::title) of the current page.
#[component]
pub async fn title(cx: &Cx) -> Result<impl View> {
    let metadata = metadata(cx).await?;
    Ok(view! {
        if let Some(title) = metadata.title() {
            <title>(title)</title>
        }
    })
}

/// Renders the [description](crate::Metadata::description) of the current
/// page.
#[component]
pub async fn description(cx: &Cx) -> Result<impl View> {
    let metadata = metadata(cx).await?;
    Ok(view! {
        if let Some(description) = metadata.description() {
            <meta name="description" content=(description)>
        }
    })
}

/// Renders the [canonical URL](crate::Metadata::canonical) of the current
/// page, always in the absolute form.
#[component]
pub async fn canonical(cx: &Cx) -> Result<impl View> {
    let metadata = metadata(cx).await?;
    let absolute = cx.with(UrlForm::Absolute);
    Ok(view! {
        if let Some(canonical) = metadata.canonical() {
            (view! { absolute => <link rel="canonical" href=(canonical)> })
        }
    })
}
