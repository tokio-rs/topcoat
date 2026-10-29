use topcoat_core::error::Result;
use topcoat_view::View;
use topcoat_view_macro::{component, view};

use crate::tag;

/// Renders the metadata of the current page as tags for `<head>`.
///
/// Render it at the start of `<head>`, because it begins with
/// [`<meta charset>`](tag::charset). To leave out a tag or write it
/// yourself, render the components of [`tag`] you need instead.
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
#[component]
pub async fn tags() -> Result<impl View> {
    Ok(view! {
        tag::charset()
        tag::viewport()
        tag::title()
        tag::description()
        tag::canonical()
    })
}
