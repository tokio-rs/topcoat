use std::{
    borrow::Cow,
    fmt::{self, Display, Write},
};

use percent_encoding::{AsciiSet, CONTROLS, utf8_percent_encode};
use serde::Serialize;
use topcoat_core::{
    base_url::base_url,
    context::Cx,
    url_form::{UrlForm, url_form},
};
use topcoat_view::{AttributeValueViewParts, NodeViewParts, PartsWriter};

use crate::{Path, PathSegment, PathSegments, request::uri};

/// The destination an [`href`] points at, resolved to the route [`Path`] the
/// URL is built from.
pub trait HrefTarget {
    /// Returns the route path the URL is built from.
    fn path<'cx>(&self, cx: &'cx Cx) -> &'cx Path;
}

impl HrefTarget for &'static Path {
    fn path<'cx>(&self, _cx: &'cx Cx) -> &'cx Path {
        self
    }
}

/// A path literal, parsed with [`Path::new`].
///
/// # Panics
///
/// Resolving panics if the string is not a well-formed path.
impl HrefTarget for &'static str {
    fn path<'cx>(&self, cx: &'cx Cx) -> &'cx Path {
        HrefTarget::path(&Path::new(self), cx)
    }
}

impl<T> HrefTarget for &T
where
    T: HrefTarget + ?Sized,
{
    fn path<'cx>(&self, cx: &'cx Cx) -> &'cx Path {
        (*self).path(cx)
    }
}

/// The characters escaped in a segment: the URL standard's path percent-encode
/// set, plus `/`, which separates segments, `%`, so that an escape a value
/// spells out survives being read back, and `\`, which a browser folds into
/// `/`.
const SEGMENT: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'/')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'`')
    .add(b'{')
    .add(b'}')
    .add(b'\\');

/// A [`Write`] that escapes what it is handed, so that a segment's `Display`
/// output lands in the URL percent-encoded without being buffered first.
struct PercentEncoded<'out>(&'out mut String);

impl Write for PercentEncoded<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.extend(utf8_percent_encode(s, SEGMENT));
        Ok(())
    }
}

/// A URL being built by [`href`].
///
/// [`query`](Self::query) adds query items, [`fragment`](Self::fragment) sets
/// the fragment, and [`relative`](Self::relative), [`absolute`](Self::absolute),
/// and [`form`](Self::form) choose the URL's form. The URL renders by using
/// the value in a view, or by calling [`resolve`](Self::resolve).
/// [`is_current`](Self::is_current) tells whether the URL points at the page
/// the current request is serving.
pub struct Href<T, P, Q, F> {
    target: T,
    params: P,
    queries: Q,
    url_form: Option<UrlForm>,
    fragment: Option<F>,
}

impl<T, P> Href<T, P, (), ()> {
    #[doc(hidden)]
    pub fn new(target: T, params: P) -> Self {
        Self {
            target,
            params,
            queries: (),
            fragment: None,
            url_form: None,
        }
    }
}

/// Generates the [`Href::query`] method for a query tuple `Q1..Qn`, which
/// grows the tuple by the added item.
macro_rules! impl_href_query_methods {
    ( $($ty:ident),* ) => {
        #[allow(non_snake_case)]
        impl<T, P, $($ty,)* F> Href<T, P, ($($ty,)*), F> {
            /// Adds one item to the URL's query string.
            #[must_use]
            pub fn query<Q>(self, query: Q) -> Href<T, P, ($($ty,)* Q,), F>
            where
                Q: Serialize,
            {
                let ($($ty,)*) = self.queries;
                Href {
                    target: self.target,
                    params: self.params,
                    queries: ($($ty,)* query,),
                    url_form: self.url_form,
                    fragment: self.fragment,
                }
            }
        }
    };
}

impl_href_query_methods!();
impl_href_query_methods!(Q1);
impl_href_query_methods!(Q1, Q2);
impl_href_query_methods!(Q1, Q2, Q3);
impl_href_query_methods!(Q1, Q2, Q3, Q4);
impl_href_query_methods!(Q1, Q2, Q3, Q4, Q5);
impl_href_query_methods!(Q1, Q2, Q3, Q4, Q5, Q6);
impl_href_query_methods!(Q1, Q2, Q3, Q4, Q5, Q6, Q7);

impl<T, P, Q, F> Href<T, P, Q, F> {
    /// Sets the URL's fragment, replacing any fragment set before.
    #[must_use]
    pub fn fragment<G>(self, fragment: G) -> Href<T, P, Q, G>
    where
        G: Display,
    {
        Href {
            target: self.target,
            params: self.params,
            queries: self.queries,
            url_form: self.url_form,
            fragment: Some(fragment),
        }
    }

    /// Renders the URL relative to the site root, like `/posts/42`.
    ///
    /// This is a shorthand for [`form`](Self::form) with
    /// [`UrlForm::Relative`].
    #[must_use]
    pub fn relative(self) -> Self {
        self.form(UrlForm::Relative)
    }

    /// Renders the URL with its scheme and host, like
    /// `https://example.com/posts/42`.
    ///
    /// This is a shorthand for [`form`](Self::form) with
    /// [`UrlForm::Absolute`].
    #[must_use]
    pub fn absolute(self) -> Self {
        self.form(UrlForm::Absolute)
    }

    /// Sets the [`UrlForm`] the URL renders in, replacing any form set
    /// before.
    ///
    /// Without a form set, the URL renders in the form [registered on the
    /// context](url_form) it resolves against: relative unless an enclosing
    /// scope, like a mail renderer, registered the absolute form.
    #[must_use]
    pub fn form(mut self, url_form: UrlForm) -> Self {
        self.url_form = Some(url_form);
        self
    }
}

impl<T, P, Q, F> Href<T, P, Q, F>
where
    T: HrefTarget,
    P: HrefParams,
    Q: HrefQueries,
    F: Display,
{
    /// Resolves the URL into its final string.
    ///
    /// The URL renders in the [`UrlForm`] set on this href, or the form
    /// registered on `cx` when none is set.
    ///
    /// # Panics
    ///
    /// Panics if the parameters do not line up with the target's path, or a
    /// query item does not serialize to a URL query string.
    #[must_use]
    pub fn resolve(&self, cx: &Cx) -> String {
        let mut buf = String::new();
        match self.url_form.unwrap_or_else(|| url_form(cx)) {
            UrlForm::Absolute => buf += base_url(cx).as_str(),
            UrlForm::Relative => {}
        }

        self.params.assign(self.target.path(cx), &mut buf);
        self.queries.assign(&mut buf);

        if let Some(fragment) = &self.fragment {
            write!(buf, "#{fragment}").unwrap();
        }

        buf
    }

    /// Returns whether this href points at the page the current request is
    /// serving.
    ///
    /// The href is current when its path, with every parameter filled in,
    /// equals the request's path, and its query matches the request's query.
    ///
    /// An href built without [`query`](Self::query) leaves the request's
    /// query out of the comparison, so a bare link to a page stays current
    /// while that page is filtered or paginated. Once a query is specified,
    /// the request's query must hold exactly the same key-value pairs,
    /// compared decoded and in any order. A specified query whose items
    /// serialize to nothing demands a request without any query.
    ///
    /// The fragment never takes part in the comparison, because a browser
    /// does not send it with a request. The [`UrlForm`] plays no role
    /// either, as only the path and the query are compared.
    ///
    /// Use it to mark the link pointing at the page being rendered:
    ///
    /// ```
    /// // src/app/posts.rs
    /// use topcoat::{
    ///     Result,
    ///     context::Cx,
    ///     router::{href, page},
    ///     view::{View, view},
    /// };
    ///
    /// #[page]
    /// async fn posts(cx: &Cx) -> Result<impl View> {
    ///     let link = href!(posts);
    ///     let current = link.is_current(cx);
    ///     Ok(view! {
    ///         <a href=(link) aria-current=(current.then_some("page"))>"Posts"</a>
    ///     })
    /// }
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the parameters do not line up with the target's path, or a
    /// query item does not serialize to a URL query string.
    #[must_use]
    pub fn is_current(&self, cx: &Cx) -> bool {
        /// Parses a query string into its decoded key-value pairs, sorted so
        /// that two queries compare equal regardless of parameter order and
        /// encoding.
        fn query_pairs(query: &str) -> Vec<(Cow<'_, str>, Cow<'_, str>)> {
            let mut pairs: Vec<_> = form_urlencoded::parse(query.as_bytes()).collect();
            pairs.sort_unstable();
            pairs
        }

        let uri = uri(cx);

        let mut path = String::new();
        self.params.assign(self.target.path(cx), &mut path);
        if uri.path() != path {
            return false;
        }
        if !Q::SPECIFIED {
            return true;
        }

        let mut query = String::new();
        self.queries.assign(&mut query);
        query_pairs(query.strip_prefix('?').unwrap_or("")) == query_pairs(uri.query().unwrap_or(""))
    }
}

/// An href used in node position renders as the URL it resolves to.
impl<T, P, Q, F> NodeViewParts for Href<T, P, Q, F>
where
    T: HrefTarget,
    P: HrefParams,
    Q: HrefQueries,
    F: Display,
{
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_string(self.resolve(cx));
    }
}

/// An href used in attribute value position renders as the URL it resolves
/// to, e.g. `href=(href(...))`.
impl<T, P, Q, F> AttributeValueViewParts for Href<T, P, Q, F>
where
    T: HrefTarget,
    P: HrefParams,
    Q: HrefQueries,
    F: Display,
{
    fn attribute_present(&self) -> bool {
        true
    }

    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_string(self.resolve(cx));
    }
}
