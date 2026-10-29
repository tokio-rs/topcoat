use std::borrow::Cow;

use topcoat_core::{
    context::{Cx, iter_context},
    error::Result,
};
use topcoat_core_macro::memoize;

use crate::{Field, Metadata, OpenGraph, OpenGraphFields, Robots, Text, Url};

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
    viewport: Field<Cow<'static, str>>,
    robots: Robots,
    theme_color: Field<Cow<'static, str>>,
    pub(crate) open_graph: OpenGraphFields,
}

impl PageMetadata {
    /// The viewport of a page when no level sets one.
    const DEFAULT_VIEWPORT: &str = "width=device-width, initial-scale=1";

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

    /// Returns the viewport of the page, or the default
    /// `width=device-width, initial-scale=1` when no level sets one.
    #[must_use]
    pub fn viewport(&self) -> Option<&str> {
        match &self.viewport {
            Field::Inherit => Some(Self::DEFAULT_VIEWPORT),
            Field::Set(viewport) => Some(viewport),
            Field::Removed => None,
        }
    }

    /// Returns the robots directives of the page.
    #[must_use]
    pub fn robots(&self) -> &Robots {
        &self.robots
    }

    /// Returns the theme color of the page.
    #[must_use]
    pub fn theme_color(&self) -> Option<&str> {
        self.theme_color.value().map(|color| &**color)
    }

    /// Returns the Open Graph metadata of the page.
    #[must_use]
    pub fn open_graph(&self) -> OpenGraph<'_> {
        OpenGraph::new(self)
    }

    /// Fills the fields that are still unset from metadata stored in the
    /// context, cloning its values.
    fn fill_from(&mut self, metadata: &Metadata) {
        self.title.fill(metadata.title.as_ref().map(Text::clone));
        self.description
            .fill(metadata.description.as_ref().map(Text::clone));
        self.canonical
            .fill(metadata.canonical.as_ref().map(Url::clone));
        self.viewport
            .fill(metadata.viewport.as_ref().map(Cow::clone));
        self.robots.fill_from(&metadata.robots);
        self.theme_color
            .fill(metadata.theme_color.as_ref().map(Cow::clone));
        if let Some(open_graph) = &metadata.open_graph {
            self.open_graph.fill_from(open_graph);
        }
    }

    /// Fills the fields that are still unset from loaded metadata, taking
    /// its values.
    fn fill_from_loaded(&mut self, metadata: Metadata) {
        self.title.fill(metadata.title);
        self.description.fill(metadata.description);
        self.canonical.fill(metadata.canonical);
        self.viewport.fill(metadata.viewport);
        self.robots.fill_from(&metadata.robots);
        self.theme_color.fill(metadata.theme_color);
        if let Some(open_graph) = metadata.open_graph {
            self.open_graph.fill_from_loaded(*open_graph);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use topcoat_core::error::Error;

    use super::*;

    /// Reads one text field of the metadata visible to `cx`.
    async fn read(cx: &Cx, field: fn(&PageMetadata) -> Option<&Text>) -> Option<String> {
        let metadata = metadata(cx).await.unwrap();
        field(metadata).map(|text| text.resolve(cx).into_owned())
    }

    #[tokio::test]
    async fn reads_each_field_from_the_closest_level_that_sets_it() {
        let cx = Cx::new()
            .with(
                Metadata::new()
                    .title("Shop")
                    .description("Everything for tea."),
            )
            .with(Metadata::new().title("Teapot"));

        assert_eq!(
            read(&cx, PageMetadata::title).await.as_deref(),
            Some("Teapot")
        );
        assert_eq!(
            read(&cx, PageMetadata::description).await.as_deref(),
            Some("Everything for tea.")
        );
    }

    #[tokio::test]
    async fn a_removed_field_hides_the_value_set_further_up() {
        let cx = Cx::new()
            .with(Metadata::new().description("Everything for tea."))
            .with(Metadata::new().no_description());

        assert_eq!(read(&cx, PageMetadata::description).await, None);
    }

    #[tokio::test]
    async fn fields_set_on_a_level_override_the_ones_it_loads() {
        let cx = Cx::new().with(
            Metadata::from_fn(async |_| {
                Ok(Metadata::new()
                    .title("Loaded")
                    .description("Loaded description."))
            })
            .title("Fixed"),
        );

        assert_eq!(
            read(&cx, PageMetadata::title).await.as_deref(),
            Some("Fixed")
        );
        assert_eq!(
            read(&cx, PageMetadata::description).await.as_deref(),
            Some("Loaded description.")
        );
    }

    #[tokio::test]
    async fn loaded_fields_override_levels_further_up() {
        let cx = Cx::new()
            .with(Metadata::new().title("Shop"))
            .with(Metadata::from_fn(async |_| {
                Ok(Metadata::new().title("Teapot"))
            }));

        assert_eq!(
            read(&cx, PageMetadata::title).await.as_deref(),
            Some("Teapot")
        );
    }

    #[tokio::test]
    async fn loaded_metadata_can_load_more() {
        let cx = Cx::new().with(Metadata::from_fn(async |_| {
            Ok(Metadata::from_fn(async |_| {
                Ok(Metadata::new().title("Nested"))
            }))
        }));

        assert_eq!(
            read(&cx, PageMetadata::title).await.as_deref(),
            Some("Nested")
        );
    }

    #[tokio::test]
    async fn returns_the_error_of_a_function() {
        let cx = Cx::new().with(Metadata::from_fn(async |_| {
            Err(Error::msg("the product is unavailable"))
        }));

        assert!(metadata(&cx).await.is_err());
    }

    #[tokio::test]
    async fn repeated_reads_run_the_functions_once() {
        static CALLS: AtomicUsize = AtomicUsize::new(0);
        let cx = Cx::new().with(Metadata::from_fn(async |_| {
            CALLS.fetch_add(1, Ordering::Relaxed);
            Ok(Metadata::new())
        }));

        metadata(&cx).await.unwrap();
        metadata(&cx).await.unwrap();

        assert_eq!(CALLS.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn reads_each_robots_directive_from_the_closest_level_that_sets_it() {
        let cx = Cx::new()
            .with(Metadata::new().robots(|robots| robots.index(false)))
            .with(Metadata::new().robots(|robots| robots.follow(false)));
        let robots = metadata(&cx).await.unwrap().robots();
        assert!(!robots.index());
        assert!(!robots.follow());

        let cx = cx.with(Metadata::new().robots(|robots| robots.index(true)));
        assert!(metadata(&cx).await.unwrap().robots().index());
    }

    #[tokio::test]
    async fn open_graph_falls_back_to_the_page_title_and_description() {
        let cx = Cx::new().with(
            Metadata::new()
                .title("Teapot")
                .description("A sturdy teapot.")
                .open_graph(|og| og.title("The best teapot")),
        );
        let page = metadata(&cx).await.unwrap();
        let og = page.open_graph();

        let resolve = |text: Option<&Text>| text.map(|text| text.resolve(&cx).into_owned());
        assert_eq!(resolve(og.title()).as_deref(), Some("The best teapot"));
        assert_eq!(
            resolve(og.description()).as_deref(),
            Some("A sturdy teapot.")
        );
    }

    #[tokio::test]
    async fn combines_open_graph_fields_from_every_level() {
        let cx = Cx::new()
            .with(Metadata::new().open_graph(|og| og.site_name("Acme").image("/acme.png")))
            .with(Metadata::new().open_graph(|og| og.image("/teapot.png").image("/lid.png")));
        let page = metadata(&cx).await.unwrap();
        let og = page.open_graph();

        assert_eq!(
            og.site_name()
                .map(|name| name.resolve(&cx).into_owned())
                .as_deref(),
            Some("Acme")
        );
        let images: Vec<_> = og
            .images()
            .iter()
            .map(|image| image.url.resolve(&cx).into_owned())
            .collect();
        assert_eq!(images, ["/teapot.png", "/lid.png"]);
    }

    #[tokio::test]
    async fn the_viewport_has_a_default_that_can_be_removed() {
        let default = Cx::new();
        let removed = Cx::new().with(Metadata::new().no_viewport());

        assert!(metadata(&default).await.unwrap().viewport().is_some());
        assert_eq!(metadata(&removed).await.unwrap().viewport(), None);
    }
}
