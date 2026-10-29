use topcoat_core::{
    context::{Cx, iter_context},
    error::Result,
};
use topcoat_core_macro::memoize;

use crate::{Field, Metadata, Text, Url};

/// Reads the metadata of the current page.
///
/// Every [`Metadata`] in the context is a level: the router's, then each
/// layout's from the outermost in, then the page's. The returned
/// [`PageMetadata`] takes each field from the closest level that sets it.
/// Within one level, fields set on the [`Metadata`] itself come before the
/// ones its [`from_fn`](Metadata::from_fn) function loads.
///
/// The result is [memoized](topcoat_core_macro::memoize): later calls in
/// the same request reuse it while the context values it read, including
/// those read by [`from_fn`](Metadata::from_fn) functions, stay the same.
///
/// # Errors
///
/// Returns the first error of a [`from_fn`](Metadata::from_fn) function.
pub async fn metadata(cx: &Cx) -> Result<&PageMetadata> {
    combine(cx).await.map_err(Clone::clone)
}

/// Combines the metadata of every level, calling their
/// [`from_fn`](Metadata::from_fn) functions.
#[memoize(as_ref)]
async fn combine(cx: &Cx) -> Result<PageMetadata> {
    let mut page = PageMetadata::default();
    for level in iter_context::<Metadata>(cx) {
        page.fill_from(level);
        let mut load = level.load.as_ref().map(|load| load.call(cx));
        while let Some(future) = load {
            let mut loaded = future.await?;
            // Metadata loaded by a function can load more in turn.
            load = loaded.load.take().map(|load| load.call(cx));
            page.fill_from_loaded(loaded);
        }
    }
    Ok(page)
}

/// The metadata of the current page, combined from every level that sets
/// some of it.
///
/// Get it with [`metadata`]. Each method returns the field from the closest
/// level that sets it. Resolve the returned value for a request, or render
/// it in a view.
#[derive(Default)]
pub struct PageMetadata {
    title: Field<Text>,
    description: Field<Text>,
    canonical: Field<Url>,
}

impl PageMetadata {
    /// Returns the title of the page.
    #[must_use]
    pub fn title(&self) -> Option<&Text> {
        self.title.value()
    }

    /// Returns the description of the page.
    #[must_use]
    pub fn description(&self) -> Option<&Text> {
        self.description.value()
    }

    /// Returns the canonical URL of the page.
    #[must_use]
    pub fn canonical(&self) -> Option<&Url> {
        self.canonical.value()
    }

    /// Fills the fields that are still unset from metadata stored in the
    /// context, cloning its values.
    fn fill_from(&mut self, metadata: &Metadata) {
        self.title.fill(metadata.title.as_ref().map(Text::clone));
        self.description
            .fill(metadata.description.as_ref().map(Text::clone));
        self.canonical
            .fill(metadata.canonical.as_ref().map(Url::clone));
    }

    /// Fills the fields that are still unset from loaded metadata, taking
    /// its values.
    fn fill_from_loaded(&mut self, metadata: Metadata) {
        self.title.fill(metadata.title);
        self.description.fill(metadata.description);
        self.canonical.fill(metadata.canonical);
    }
}
