use topcoat_core::{context::Cx, error::Result};
use topcoat_view::View;
use topcoat_view_macro::{component, view};

use crate::{metadata, tag};

/// Renders the metadata of the current page as tags for `<head>`.
///
/// Render it at the start of `<head>`, because it begins with
/// [`<meta charset>`](tag::charset). It reads the metadata once and passes
/// it to every component of [`tag`]. To leave out a tag or write it
/// yourself, render the components you need instead.
///
/// ```
/// use topcoat::{
///     Result,
///     metadata::{self, Metadata},
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
///                 metadata::tags()
///             </head>
///             <body>(slot)</body>
///         </html>
///     })
/// }
/// ```
///
/// # Errors
///
/// Returns the error of [`metadata`] when reading the metadata fails.
#[component]
pub async fn tags(cx: &Cx) -> Result<impl View> {
    let metadata = metadata(cx).await?;
    Ok(view! {
        tag::charset()
        tag::viewport(metadata: metadata)
        tag::title(metadata: metadata)
        tag::description(metadata: metadata)
        tag::canonical(metadata: metadata)
    })
}
