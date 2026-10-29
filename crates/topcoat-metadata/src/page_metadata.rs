use std::borrow::Cow;

use topcoat_core::{
    context::{Cx, iter_context},
    error::Result,
};
use topcoat_core_macro::memoize;

use crate::{
    Metadata, OpenGraph, OpenGraphFields, Robots, Text, Twitter, TwitterFields, Url,
    field::{Field, Fill, fill_fields},
};

/// Collects the current page's metadata from its context.
///
/// [`Metadata`] can come from the router, enclosing layouts, or the page.
/// Each field uses the closest value, so page settings override layout
/// settings, and inner layouts override outer ones. Within a single
/// [`Metadata`] value, fields set directly override fields loaded by its
/// [`from_fn`](Metadata::from_fn) function. Removing a field also prevents
/// it from inheriting a value.
///
/// The result is [cached for the request](topcoat_core_macro::memoize).
/// Later calls reuse it as long as the context values it depends on stay
/// the same. This includes values read by [`from_fn`](Metadata::from_fn).
///
/// # Errors
///
/// Stops and returns the first error from a [`from_fn`](Metadata::from_fn) call.
pub async fn metadata(cx: &Cx) -> Result<&PageMetadata> {
    combine(cx).await.map_err(Clone::clone)
}

/// Merges all metadata entries, including values loaded by their
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
            page.fill_from_owned(loaded);
        }
    }
    Ok(page)
}

/// The page's metadata after combining its own settings with inherited ones.
///
/// Call [`metadata`] to get this value. Its methods return the fields chosen
/// from the closest metadata entries. You can resolve text and URL values
/// for the request or render them directly in a view.
#[derive(Default)]
pub struct PageMetadata {
    title: Field<Text>,
    description: Field<Text>,
    canonical: Field<Url>,
    viewport: Field<Cow<'static, str>>,
    robots: Robots,
    theme_color: Field<Cow<'static, str>>,
    pub(crate) open_graph: OpenGraphFields,
    pub(crate) twitter: TwitterFields,
}

impl PageMetadata {
    /// The viewport used when no metadata entry supplies one.
    const DEFAULT_VIEWPORT: &str = "width=device-width, initial-scale=1";

    /// Returns the page's title, if set.
    #[must_use]
    pub fn title(&self) -> Option<&Text> {
        self.title.value()
    }

    /// Returns the page's description, if set.
    #[must_use]
    pub fn description(&self) -> Option<&Text> {
        self.description.value()
    }

    /// Returns the page's preferred URL, if set.
    #[must_use]
    pub fn canonical(&self) -> Option<&Url> {
        self.canonical.value()
    }

    /// Returns the viewport setting. If none is set, it defaults to
    /// `width=device-width, initial-scale=1`. An explicitly removed viewport
    /// returns `None`.
    #[must_use]
    pub fn viewport(&self) -> Option<&str> {
        match &self.viewport {
            Field::Inherit => Some(Self::DEFAULT_VIEWPORT),
            Field::Set(viewport) => Some(viewport),
            Field::Removed => None,
        }
    }

    /// Returns the page's instructions for search engines.
    #[must_use]
    pub fn robots(&self) -> &Robots {
        &self.robots
    }

    /// Returns the suggested browser theme color, if set.
    #[must_use]
    pub fn theme_color(&self) -> Option<&str> {
        self.theme_color.value().map(|color| &**color)
    }

    /// Provides access to the page's Open Graph values.
    #[must_use]
    pub fn open_graph(&self) -> OpenGraph<'_> {
        OpenGraph::new(self)
    }

    /// Provides access to the page's X preview settings.
    #[must_use]
    pub fn twitter(&self) -> Twitter<'_> {
        Twitter::new(self)
    }
}

fill_fields!(PageMetadata from Metadata {
    title,
    description,
    canonical,
    viewport,
    robots,
    theme_color,
    open_graph,
    twitter,
});

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use topcoat_core::error::Error;

    use super::*;
    use crate::TwitterCard;

    /// Resolves a selected text field from the metadata in `cx`.
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
    async fn the_twitter_card_defaults_to_a_large_image_when_there_are_images() {
        let without_images = Cx::new();
        let with_images = Cx::new().with(Metadata::new().open_graph(|og| og.image("/teapot.png")));
        let explicit = with_images.with(Metadata::new().twitter(|x| x.card(TwitterCard::Summary)));

        let card = async |cx| metadata(cx).await.unwrap().twitter().card();
        assert_eq!(card(&without_images).await, TwitterCard::Summary);
        assert_eq!(card(&with_images).await, TwitterCard::SummaryLargeImage);
        assert_eq!(card(&explicit).await, TwitterCard::Summary);
    }

    #[tokio::test]
    async fn the_viewport_has_a_default_that_can_be_removed() {
        let default = Cx::new();
        let removed = Cx::new().with(Metadata::new().no_viewport());

        assert!(metadata(&default).await.unwrap().viewport().is_some());
        assert_eq!(metadata(&removed).await.unwrap().viewport(), None);
    }
}
