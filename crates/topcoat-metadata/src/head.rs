use topcoat_core::{context::Cx, error::Result};
use topcoat_view::View;
use topcoat_view_macro::{component, view};

use crate::{metadata, tag};

/// Writes the current page's metadata as HTML tags inside `<head>`.
///
/// Place this component first in `<head>` so its
/// [`<meta charset>`](tag::charset) tag appears early in the document.
/// It reads the page metadata once and renders the components in [`tag`].
/// To choose which tags to include or supply your own markup, use those
/// components individually.
///
/// ```
/// use topcoat::{
///     Result,
///     metadata::Metadata,
///     router::{Slot, layout},
///     view::{View, view},
/// };
///
/// #[layout("/", with(Metadata::new().title("Acme")))]
/// async fn shell(slot: Slot<'_>) -> Result<impl View> {
///     Ok(view! {
///         <!DOCTYPE html>
///         <html>
///             <head>
///                 topcoat::metadata::tags()
///             </head>
///             <body>(slot)</body>
///         </html>
///     })
/// }
/// ```
///
/// # Errors
///
/// Returns any error from loading the page's [`metadata`].
#[component]
pub async fn tags(cx: &Cx) -> Result<impl View> {
    let metadata = metadata(cx).await?;
    Ok(view! {
        tag::charset()
        tag::viewport(metadata: metadata)
        tag::title(metadata: metadata)
        tag::description(metadata: metadata)
        tag::canonical(metadata: metadata)
        tag::robots(metadata: metadata)
        tag::theme_color(metadata: metadata)
        tag::open_graph(metadata: metadata)
        tag::twitter(metadata: metadata)
    })
}
