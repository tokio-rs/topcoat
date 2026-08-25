#![doc = include_str!("../../docs/content/sitemap.md")]

use std::{borrow::Cow, fmt, time::SystemTime};

use http::header::{CONTENT_TYPE, HeaderValue};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use topcoat_core::{base_url::base_url, context::Cx, error::Result};
use topcoat_view::{Formatter, HtmlContext};

use crate::{
    Body,
    response::{IntoResponse, Response},
};

/// An XML sitemap response, assembled URL by URL.
///
/// A sitemap lists the pages of a site for crawlers. Build one by adding
/// entries with [`url`](Sitemap::url) and [`urls`](Sitemap::urls), then
/// return it from a route serving `/sitemap.xml`. The response is sent with
/// `Content-Type: application/xml`.
///
/// An entry holding a root-relative path is resolved against the base URL
/// registered on the router with `.base_url(...)`, because the sitemap
/// format requires absolute URLs. An entry that is already an absolute
/// `http` or `https` URL is used as is.
///
/// # Panics
///
/// Converting the sitemap into a response panics if an entry holds a
/// root-relative path and no base URL is registered.
///
/// # Examples
///
/// ```rust
/// use topcoat::{
///     Result,
///     router::{
///         content::sitemap::{ChangeFrequency, Sitemap, SitemapUrl},
///         route,
///     },
/// };
///
/// #[route(GET "/sitemap.xml")]
/// async fn sitemap() -> Result<Sitemap> {
///     Ok(Sitemap::new()
///         .url("/")
///         .url(SitemapUrl::new("/about").change_frequency(ChangeFrequency::Monthly)))
/// }
/// ```
#[derive(Clone, Debug, Default)]
#[must_use]
pub struct Sitemap {
    urls: Vec<SitemapUrl>,
}

impl Sitemap {
    /// Creates a sitemap without any entries.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an entry.
    ///
    /// Accepts a location string or a [`SitemapUrl`] carrying the optional
    /// fields.
    pub fn url(mut self, url: impl Into<SitemapUrl>) -> Self {
        self.urls.push(url.into());
        self
    }

    /// Adds every entry of an iterator, such as one built from the rows of
    /// a database query.
    pub fn urls<I>(mut self, urls: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<SitemapUrl>,
    {
        self.urls.extend(urls.into_iter().map(Into::into));
        self
    }

    /// Serializes the sitemap into the XML document, resolving relative
    /// locations against the registered base URL.
    fn serialize(&self, cx: &Cx) -> Result<String> {
        let mut xml = String::from(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
        );
        let mut f = Formatter::new(&mut xml);
        for url in &self.urls {
            let location = if is_absolute(&url.location) {
                Cow::Borrowed(url.location.as_str())
            } else {
                Cow::Owned(base_url(cx).join(&url.location))
            };
            f.write_str("<url><loc>");
            // Text-node escaping applies to XML content just as to HTML.
            HtmlContext::Text.writer(&mut f).write_str(&location);
            f.write_str("</loc>");
            if let Some(last_modified) = url.last_modified {
                let last_modified = OffsetDateTime::from(last_modified)
                    .format(&Rfc3339)
                    .map_err(|_| {
                        InvalidSitemapError::new(
                            "a last modified time is outside the representable range",
                        )
                    })?;
                f.write_str("<lastmod>");
                f.write_str(&last_modified);
                f.write_str("</lastmod>");
            }
            if let Some(change_frequency) = url.change_frequency {
                f.write_str("<changefreq>");
                f.write_str(change_frequency.as_str());
                f.write_str("</changefreq>");
            }
            if let Some(priority) = url.priority {
                if !(0.0..=1.0).contains(&priority) {
                    return Err(
                        InvalidSitemapError::new("a priority must be between 0.0 and 1.0").into(),
                    );
                }
                f.write_str("<priority>");
                f.write_str(&priority.to_string());
                f.write_str("</priority>");
            }
            f.write_str("</url>\n");
        }
        f.write_str("</urlset>\n");
        Ok(xml)
    }
}

impl IntoResponse for Sitemap {
    async fn into_response(self, cx: &Cx) -> Result<Response> {
        (
            [(CONTENT_TYPE, HeaderValue::from_static("application/xml"))],
            Body::from(self.serialize(cx)?),
        )
            .into_response(cx)
            .await
    }
}

/// One [`Sitemap`] entry, assembled field by field.
///
/// The location is required and set at construction; every other field is
/// optional and replaced by the builder method that sets it.
///
/// # Examples
///
/// ```rust
/// use std::time::SystemTime;
///
/// use topcoat::router::content::sitemap::{ChangeFrequency, SitemapUrl};
///
/// let url = SitemapUrl::new("/posts/42")
///     .last_modified(SystemTime::now())
///     .change_frequency(ChangeFrequency::Weekly)
///     .priority(0.8);
/// ```
#[derive(Clone, Debug)]
#[must_use]
pub struct SitemapUrl {
    location: String,
    last_modified: Option<SystemTime>,
    change_frequency: Option<ChangeFrequency>,
    priority: Option<f32>,
}

impl SitemapUrl {
    /// Creates an entry for the page at `location`: a root-relative path
    /// resolved against the registered base URL, or an absolute `http` or
    /// `https` URL used as is.
    pub fn new(location: impl Into<String>) -> Self {
        Self {
            location: location.into(),
            last_modified: None,
            change_frequency: None,
            priority: None,
        }
    }

    /// Sets the time the page was last modified.
    ///
    /// Accepts anything convertible into a [`SystemTime`], which covers the
    /// timestamp types of the common date and time crates.
    pub fn last_modified(mut self, last_modified: impl Into<SystemTime>) -> Self {
        self.last_modified = Some(last_modified.into());
        self
    }

    /// Sets how frequently the page is likely to change.
    pub fn change_frequency(mut self, change_frequency: ChangeFrequency) -> Self {
        self.change_frequency = Some(change_frequency);
        self
    }

    /// Sets the priority of the page relative to the other pages of the
    /// site, from `0.0` to `1.0`. Crawlers treat an entry without a
    /// priority as `0.5`.
    ///
    /// A value outside the range is rejected when the sitemap is converted
    /// into a response.
    pub fn priority(mut self, priority: f32) -> Self {
        self.priority = Some(priority);
        self
    }
}

impl From<&str> for SitemapUrl {
    fn from(location: &str) -> Self {
        Self::new(location)
    }
}

impl From<String> for SitemapUrl {
    fn from(location: String) -> Self {
        Self::new(location)
    }
}

/// How frequently a page is likely to change, hinting crawlers how often to
/// revisit it.
///
/// [`Always`](ChangeFrequency::Always) describes a page that changes on
/// every access, [`Never`](ChangeFrequency::Never) an archived page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeFrequency {
    /// The page changes on every access.
    Always,
    /// The page changes about once an hour.
    Hourly,
    /// The page changes about once a day.
    Daily,
    /// The page changes about once a week.
    Weekly,
    /// The page changes about once a month.
    Monthly,
    /// The page changes about once a year.
    Yearly,
    /// The page is archived and never changes.
    Never,
}

impl ChangeFrequency {
    /// The value as it appears in a `<changefreq>` element.
    fn as_str(self) -> &'static str {
        match self {
            Self::Always => "always",
            Self::Hourly => "hourly",
            Self::Daily => "daily",
            Self::Weekly => "weekly",
            Self::Monthly => "monthly",
            Self::Yearly => "yearly",
            Self::Never => "never",
        }
    }
}

/// The error produced when a [`Sitemap`] holds a field whose value cannot
/// be represented in the sitemap format.
#[derive(Debug)]
pub struct InvalidSitemapError {
    description: &'static str,
}

impl InvalidSitemapError {
    fn new(description: &'static str) -> Self {
        Self { description }
    }

    /// Returns the description of the invalid field.
    #[must_use]
    pub fn description(&self) -> &str {
        self.description
    }
}

impl fmt::Display for InvalidSitemapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid sitemap: {}", self.description)
    }
}

impl std::error::Error for InvalidSitemapError {}

/// Returns `true` if the location is an absolute `http` or `https` URL.
fn is_absolute(location: &str) -> bool {
    ["http://", "https://"].iter().any(|scheme| {
        location
            .get(..scheme.len())
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case(scheme))
    })
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use topcoat_core::{base_url::BaseUrl, context::CxTestBuilder};

    use super::*;
    use crate::to_bytes;

    /// Builds a `Cx` with `https://example.com` registered as the base URL.
    fn cx() -> Cx {
        CxTestBuilder::new()
            .app_context(BaseUrl::new("https://example.com").expect("a valid base URL"))
            .build()
    }

    #[test]
    fn an_empty_sitemap_is_an_empty_urlset() {
        assert_eq!(
            Sitemap::new().serialize(&cx()).unwrap(),
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n\
             </urlset>\n"
        );
    }

    #[test]
    fn every_field_is_serialized() {
        let url = SitemapUrl::new("/posts/1")
            .last_modified(SystemTime::UNIX_EPOCH + Duration::from_hours(24))
            .change_frequency(ChangeFrequency::Weekly)
            .priority(0.8);
        let xml = Sitemap::new().url(url).serialize(&cx()).unwrap();
        assert!(xml.contains(
            "<url>\
             <loc>https://example.com/posts/1</loc>\
             <lastmod>1970-01-02T00:00:00Z</lastmod>\
             <changefreq>weekly</changefreq>\
             <priority>0.8</priority>\
             </url>"
        ));
    }

    #[test]
    fn relative_locations_resolve_against_the_base_url() {
        let xml = Sitemap::new()
            .url("/")
            .url("about")
            .serialize(&cx())
            .unwrap();
        assert!(xml.contains("<loc>https://example.com/</loc>"));
        assert!(xml.contains("<loc>https://example.com/about</loc>"));
    }

    #[test]
    fn absolute_locations_are_used_as_is() {
        let xml = Sitemap::new()
            .url("https://cdn.example.com/page")
            .url("HTTP://example.com/UPPER")
            .serialize(&cx())
            .unwrap();
        assert!(xml.contains("<loc>https://cdn.example.com/page</loc>"));
        assert!(xml.contains("<loc>HTTP://example.com/UPPER</loc>"));
    }

    #[test]
    fn urls_adds_every_entry_of_an_iterator() {
        let xml = Sitemap::new().urls(["/a", "/b"]).serialize(&cx()).unwrap();
        assert!(xml.contains("<loc>https://example.com/a</loc>"));
        assert!(xml.contains("<loc>https://example.com/b</loc>"));
    }

    #[test]
    fn reserved_characters_in_locations_are_escaped() {
        // Quotes are legal in XML text content and pass through unescaped.
        let xml = Sitemap::new()
            .url("/search?q=<a>&sort=\"new\"")
            .serialize(&cx())
            .unwrap();
        assert!(xml.contains("<loc>https://example.com/search?q=&lt;a&gt;&amp;sort=\"new\"</loc>"));
    }

    #[test]
    fn a_priority_outside_the_range_is_an_error() {
        for priority in [-0.1, 1.1] {
            let error = Sitemap::new()
                .url(SitemapUrl::new("/").priority(priority))
                .serialize(&cx())
                .unwrap_err();
            assert!(error.downcast_ref::<InvalidSitemapError>().is_some());
        }
    }

    #[test]
    #[should_panic(expected = "attempted to access the base URL")]
    fn a_relative_location_without_a_base_url_panics() {
        let _ = Sitemap::new().url("/").serialize(&Cx::default());
    }

    #[tokio::test]
    async fn into_response_sets_the_xml_content_type() {
        let response = Sitemap::new()
            .url("/")
            .into_response(&cx())
            .await
            .expect("response builds");

        assert_eq!(
            response
                .headers()
                .get(CONTENT_TYPE)
                .map(HeaderValue::as_bytes),
            Some(b"application/xml".as_slice())
        );

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("reading the response body");
        assert!(body.starts_with(b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
    }
}
