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

/// A value for one path parameter, named after the parameter it fills.
///
/// The `path_param!` macro implements this trait for the types it declares.
/// An [`href`] hands each provided value the [`HrefSegments`] of the path
/// parameter carrying the same name, and the value pushes what it fills that
/// parameter with: one segment for a regular parameter, one per element for a
/// catch-all.
///
/// Pushing a segment does not render it. A URL renders its segments with
/// [`Display`], so that is what [`HrefParams`] requires of the
/// [`Segment`](Self::Segment) type. A parameter whose type cannot be rendered
/// still declares; only building a URL from it is rejected.
pub trait HrefParam {
    /// The type of one segment this value fills its parameter with.
    type Segment: ?Sized;

    /// The name of the path parameter this value fills.
    fn name(&self) -> &str;

    /// Pushes the segments this value fills its parameter with.
    fn segments(&self, segments: &mut HrefSegments<'_, Self::Segment>);
}

impl<T> HrefParam for &T
where
    T: HrefParam,
{
    type Segment = T::Segment;

    fn name(&self) -> &str {
        (*self).name()
    }

    fn segments(&self, segments: &mut HrefSegments<'_, Self::Segment>) {
        (*self).segments(segments);
    }
}

/// The segments an [`HrefParam`] fills its path parameter with.
///
/// [`push`](Self::push) appends one segment; a catch-all pushes one per
/// element it spans. The separating `/` and the escaping are written here, so
/// a value never spells out either itself.
pub struct HrefSegments<'out, S: ?Sized> {
    write: &'out mut dyn FnMut(&S),
    count: usize,
}

impl<S: ?Sized> HrefSegments<'_, S> {
    /// Appends one path segment, percent-encoded.
    ///
    /// The segment is written with [`Display`] and lands in the URL escaped,
    /// so a value that contains `/`, `?`, or `#` fills exactly this one
    /// segment instead of reshaping the URL around it.
    ///
    /// # Panics
    ///
    /// Panics if the segment's [`Display`] implementation fails, or if it
    /// renders to nothing, `.`, or `..`. A browser resolves those against the
    /// rest of the path instead of reading them as one segment, and encoding
    /// them does not take that meaning away, so they are rejected.
    pub fn push(&mut self, segment: &S) {
        self.count += 1;
        (self.write)(segment);
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

/// The path parameters of an [`href`]: a tuple of up to eight [`HrefParam`]
/// values, in the order the path declares its parameters.
///
/// Every value renders the segments it pushes, so each one's
/// [`Segment`](HrefParam::Segment) type must implement [`Display`].
pub trait HrefParams {
    /// Writes `path` into `out` with every parameter filled in, each segment
    /// percent-encoded.
    ///
    /// # Panics
    ///
    /// Panics if the values do not line up with the path's parameters: a name
    /// mismatch, a value the path declares no parameter for, a parameter no
    /// value fills, or a value that fills its parameter with the wrong number
    /// of segments. Panics as well if a segment renders to nothing, `.`, or
    /// `..`, which address another path instead of filling a segment.
    fn assign(&self, path: &Path, out: &mut String);
}

/// Generates the `HrefParams` impl for a tuple of parameters `P1..Pn`.
///
/// The parameters fill the path's capturing segments in order: the walk copies
/// static segments into the output, skips group segments, and lets each
/// parameter push its segments where the path declares the parameter of the
/// same name.
macro_rules! impl_href_params_tuples {
    ( $($ty:ident),* ) => {
        #[allow(non_snake_case, unused_mut, unused_variables)]
        impl<$($ty,)*> HrefParams for ($($ty,)*)
        where
            $(
                $ty: HrefParam,
                <$ty as HrefParam>::Segment: Display,
            )*
        {
            fn assign(&self, path: &Path, out: &mut String) {
                let start = out.len();
                let ($($ty,)*) = self;
                let mut segments = path.segments();
                $(
                    let (name, catch_all) = next_param(&mut segments, path, out);
                    assert_eq!(
                        name,
                        $ty.name(),
                        "provided parameter \"{}\" does not fill path parameter \
                         \"{name}\" in `{path}`",
                        $ty.name(),
                    );
                    push_segments($ty, name, catch_all, path, out);
                )*
                write_remaining(segments, path, out);
                // A path with no URL segments, like the root or a group-only
                // path, still addresses `/`.
                if out.len() == start {
                    out.push('/');
                }
            }
        }
    };
}

impl_href_params_tuples!();
impl_href_params_tuples!(P1);
impl_href_params_tuples!(P1, P2);
impl_href_params_tuples!(P1, P2, P3);
impl_href_params_tuples!(P1, P2, P3, P4);
impl_href_params_tuples!(P1, P2, P3, P4, P5);
impl_href_params_tuples!(P1, P2, P3, P4, P5, P6);
impl_href_params_tuples!(P1, P2, P3, P4, P5, P6, P7);
impl_href_params_tuples!(P1, P2, P3, P4, P5, P6, P7, P8);

/// Advances to the path's next capturing segment and returns the name it
/// captures under and whether it is a catch-all, copying the static segments
/// crossed on the way into `out`.
///
/// # Panics
///
/// Panics if the path declares no further capturing segment.
fn next_param<'path>(
    segments: &mut PathSegments<'path>,
    path: &Path,
    out: &mut String,
) -> (&'path str, bool) {
    loop {
        match segments.next() {
            Some(PathSegment::Group(_)) => {}
            Some(PathSegment::Static(segment)) => {
                out.push('/');
                out.push_str(segment);
            }
            Some(PathSegment::Param(name)) => return (name, false),
            Some(PathSegment::CatchAll(name)) => return (name, true),
            None => panic!("`{path}` declares fewer parameters than the href provides"),
        }
    }
}

/// Lets `param` push the segments it fills `name` with, rendering each one
/// percent-encoded into `out` and checking that the parameter filled as much
/// of the path as it stands for.
///
/// # Panics
///
/// Panics if a segment renders to nothing, `.`, or `..`, none of which fill a
/// segment of their own. Panics if a regular parameter does not push exactly
/// one segment, or a catch-all pushes none: a catch-all matches at least one
/// segment, so an empty one addresses a path its route does not serve.
fn push_segments<P>(param: &P, name: &str, catch_all: bool, path: &Path, out: &mut String)
where
    P: HrefParam,
    P::Segment: Display,
{
    let mut write = |segment: &P::Segment| {
        let start = out.len();
        out.push('/');
        write!(PercentEncoded(&mut *out), "{segment}").unwrap();

        // The URL is read back segment by segment, and a browser resolves an
        // empty, current, or parent segment against the path around it. No
        // encoding takes that meaning away, so the value is rejected instead.
        let rendered = &out[start + 1..];
        assert!(
            !rendered.is_empty(),
            "parameter \"{name}\" in `{path}` was given an empty segment"
        );
        assert!(
            !matches!(rendered, "." | ".."),
            "parameter \"{name}\" in `{path}` was given \"{rendered}\", which \
             addresses another path instead of filling a segment"
        );
    };
    let mut segments = HrefSegments {
        write: &mut write,
        count: 0,
    };
    param.segments(&mut segments);
    let count = segments.count;

    if catch_all {
        assert!(
            count > 0,
            "catch-all parameter \"{name}\" in `{path}` was given no segment"
        );
    } else {
        assert!(
            count == 1,
            "parameter \"{name}\" fills one segment of `{path}`, but was given {count}"
        );
    }
}

/// Copies the path's remaining static segments into `out` once every provided
/// parameter is assigned.
///
/// # Panics
///
/// Panics if the path declares a capturing segment that no parameter fills.
fn write_remaining(segments: PathSegments<'_>, path: &Path, out: &mut String) {
    for segment in segments {
        match segment {
            PathSegment::Group(_) => {}
            PathSegment::Static(segment) => {
                out.push('/');
                out.push_str(segment);
            }
            PathSegment::Param(name) | PathSegment::CatchAll(name) => {
                panic!("no value provided for path parameter \"{name}\" in `{path}`")
            }
        }
    }
}

/// The query of an [`href`]: a tuple of up to eight [`Serialize`] items,
/// collected by [`Href::query`].
pub trait HrefQueries {
    /// Whether a query was specified: `true` when the tuple holds at least
    /// one item, even one that serializes to nothing.
    const SPECIFIED: bool;

    /// Appends the items, serialized and concatenated into one query string,
    /// to the URL in `out`.
    ///
    /// # Panics
    ///
    /// Panics if an item does not serialize to a URL query string.
    fn assign(&self, out: &mut String);
}

/// The empty tuple: no query was specified, and nothing is appended.
impl HrefQueries for () {
    const SPECIFIED: bool = false;

    fn assign(&self, _out: &mut String) {}
}

/// Generates the `HrefQueries` impl for a non-empty tuple of query items
/// `Q1..Qn`.
///
/// Each item serializes to a query string on its own; the non-empty results
/// are concatenated, the first behind `?` and the rest behind `&`.
macro_rules! impl_href_queries_tuples {
    ( $($ty:ident),+ ) => {
        #[allow(non_snake_case, unused_assignments)]
        impl<$($ty,)+> HrefQueries for ($($ty,)+)
        where
            $($ty: Serialize,)+
        {
            const SPECIFIED: bool = true;

            fn assign(&self, out: &mut String) {
                let ($($ty,)+) = self;
                let mut separator = '?';
                $(
                    if write_query($ty, separator, out) {
                        separator = '&';
                    }
                )+
            }
        }
    };
}

impl_href_queries_tuples!(Q1);
impl_href_queries_tuples!(Q1, Q2);
impl_href_queries_tuples!(Q1, Q2, Q3);
impl_href_queries_tuples!(Q1, Q2, Q3, Q4);
impl_href_queries_tuples!(Q1, Q2, Q3, Q4, Q5);
impl_href_queries_tuples!(Q1, Q2, Q3, Q4, Q5, Q6);
impl_href_queries_tuples!(Q1, Q2, Q3, Q4, Q5, Q6, Q7);
impl_href_queries_tuples!(Q1, Q2, Q3, Q4, Q5, Q6, Q7, Q8);

/// Serializes one query item behind `separator` into `out`, returning whether
/// the item produced any query string at all.
///
/// # Panics
///
/// Panics if the item does not serialize to a URL query string.
fn write_query<Q: Serialize>(query: &Q, separator: char, out: &mut String) -> bool {
    let query = serde_urlencoded::to_string(query)
        .unwrap_or_else(|error| panic!("query item does not serialize to a query string: {error}"));
    if query.is_empty() {
        return false;
    }
    out.push(separator);
    out.push_str(&query);
    true
}

/// Parses a query string into its decoded key-value pairs, sorted so that two
/// queries compare equal regardless of parameter order and encoding.
fn query_pairs(query: &str) -> Vec<(Cow<'_, str>, Cow<'_, str>)> {
    let mut pairs: Vec<_> = form_urlencoded::parse(query.as_bytes()).collect();
    pairs.sort_unstable();
    pairs
}

/// Turns a page or route into an URL string.
///
/// The first parameter, `target`, is the route handler the URL should be
/// pointing to. The URL is built from the path the handler is mounted at, so
/// it stays in sync when the route moves. A [`Path`] or a plain path string
/// works as well.
///
/// Inside a handler's own body its name refers to the handler function, so a
/// handler linking to itself names its marker as a type, e.g. `posts {}`, or
/// uses the [`href!`](macro@crate::href) macro, which does that on its own.
///
/// `params` fills in the path's parameters: one `path_param!` value per
/// parameter, in the order the path declares them, passed as a tuple. A path
/// without parameters takes `()`. Each value is written with [`Display`] and
/// percent-encoded, so a parameter declared with a type, like
/// `path_param!(post_id: u64)`, needs that type to implement [`Display`].
///
/// The result can be extended with a [`query`](Href::query) string, a
/// [`fragment`](Href::fragment), and an [`absolute`](Href::absolute) or
/// [`relative`](Href::relative) form. Use it directly in a view to render
/// the URL, or call [`resolve`](Href::resolve) to get the string, e.g. for
/// a redirect. [`is_current`](Href::is_current) tells whether the URL points
/// at the page the current request is serving, e.g. to mark the link to it in
/// a nav.
///
/// ```
/// use serde::Serialize;
/// use topcoat::{
///     Result,
///     context::Cx,
///     router::{href, page, path_param},
///     view::view,
/// };
///
/// path_param!(post_id: u64);
///
/// #[derive(Serialize)]
/// struct Pagination {
///     page: u32,
/// }
///
/// #[page("/posts")]
/// async fn posts(cx: &Cx) -> Result {
///     let comments = href("/posts/{post_id}", (PostId(5),))
///         .query(Pagination { page: 2 })
///         .fragment("comments");
///     view! {
///         <a href=(comments)>"Comments of the fifth post"</a>
///     }
/// }
/// ```
///
/// The [`href!`](macro@crate::href) macro builds the same URL with the
/// parameters listed as plain arguments instead of a tuple.
#[must_use]
pub fn href<T, P>(target: T, params: P) -> Href<T, P, (), &'static str>
where
    T: HrefTarget,
    P: HrefParams,
{
    Href {
        target,
        params,
        queries: (),
        url_form: None,
        fragment: None,
    }
}

/// Turns a page or route into an URL string.
///
/// The first argument is the route handler the URL should be pointing to.
/// The URL is built from the path the handler is mounted at, so it stays in
/// sync when the route moves. A handler links to itself as well, so a page
/// can point at its own path. A [`Path`] or a plain path string works too,
/// but a bare path always names a handler, so a [`Path`] held in a constant
/// goes through the [`href`] function instead.
///
/// Every further argument fills in one of the path's parameters, with one
/// `path_param!` value per parameter in the order the path declares them, so
/// a link to a post reads as `href!(post, PostId(post.id))`. Each value is
/// written with [`Display`] and percent-encoded, so a parameter declared with
/// a type, like `path_param!(post_id: u64)`, needs that type to implement
/// [`Display`].
///
/// The result can be extended with a [`query`](Href::query) string, a
/// [`fragment`](Href::fragment), and an [`absolute`](Href::absolute) or
/// [`relative`](Href::relative) form. Use it directly in a view to render
/// the URL, or call [`resolve`](Href::resolve) to get the string, e.g. for
/// a redirect. [`is_current`](Href::is_current) tells whether the URL points
/// at the page the current request is serving, e.g. to mark the link to it in
/// a nav.
///
/// ```
/// use serde::Serialize;
/// use topcoat::{
///     Result,
///     context::Cx,
///     router::{
///         error::{SeeOther, bad_request, see_other},
///         href, page, path_param, route,
///     },
///     view::view,
/// };
///
/// path_param!(post_id: u64, error = bad_request);
///
/// #[derive(Serialize)]
/// struct Pagination {
///     page: u32,
/// }
///
/// #[page("/posts")]
/// async fn posts(cx: &Cx) -> Result {
///     view! {
///         <a href=(href!(post, PostId(1)))>"The first post"</a>
///         <a href=(href!(post, PostId(1)).fragment("comments"))>"Its comments"</a>
///         <a href=(href!(posts).query(Pagination { page: 2 }))>"Next page"</a>
///     }
/// }
///
/// #[page("/posts/{post_id}")]
/// async fn post(cx: &Cx) -> Result {
///     let post_id = path_param::<PostId>(cx)?;
///
///     view! {
///         <form method="post" action=(href!(publish, PostId(*post_id)))>
///             <button>"Publish"</button>
///         </form>
///     }
/// }
///
/// #[route(POST "/posts/{post_id}/publish")]
/// async fn publish(cx: &Cx) -> Result<SeeOther> {
///     let post_id = path_param::<PostId>(cx)?;
///
///     Ok(see_other(href!(post, PostId(*post_id)).resolve(cx)))
/// }
/// ```
///
/// The macro is a thin wrapper around the [`href`] function, which takes the
/// parameters as a tuple instead.
#[macro_export]
macro_rules! href {
    // A bare path names the marker a `#[page]` or `#[route]` expands to, and is
    // resolved as a type: inside a handler's own body the re-emitted function
    // shadows the marker in the value namespace, so only the type namespace
    // reaches it. Every other target is an expression.
    ( $( $target:ident )::+ $(, $param:expr)* $(,)? ) => {
        $crate::href(
            <$($target)::+ as ::core::default::Default>::default(),
            ($($param,)*),
        )
    };
    ( $target:expr $(, $param:expr)* $(,)? ) => {
        $crate::href($target, ($($param,)*))
    };
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
    /// use topcoat::{
    ///     Result,
    ///     context::Cx,
    ///     router::{href, page},
    ///     view::view,
    /// };
    ///
    /// #[page("/posts")]
    /// async fn posts(cx: &Cx) -> Result {
    ///     let link = href!(posts);
    ///     let current = link.is_current(cx);
    ///     view! {
    ///         <a href=(link) aria-current=(current.then_some("page"))>"Posts"</a>
    ///     }
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

#[cfg(test)]
mod tests {
    use topcoat_core::{base_url::BaseUrl, context::CxTestBuilder};

    use super::*;

    /// An href parameter with a fixed name, filling one segment.
    struct Param(&'static str, &'static str);

    impl HrefParam for Param {
        type Segment = str;

        fn name(&self) -> &str {
            self.0
        }

        fn segments(&self, segments: &mut HrefSegments<'_, str>) {
            segments.push(self.1);
        }
    }

    /// An href parameter with a fixed name, filling one segment per element.
    struct Tail(&'static str, &'static [&'static str]);

    impl HrefParam for Tail {
        type Segment = str;

        fn name(&self) -> &str {
            self.0
        }

        fn segments(&self, segments: &mut HrefSegments<'_, str>) {
            for segment in self.1 {
                segments.push(segment);
            }
        }
    }

    /// Builds a context carrying `https://example.com` as its base URL.
    fn cx_with_base_url() -> Cx {
        CxTestBuilder::new()
            .app_context(BaseUrl::new("https://example.com").expect("a valid base URL"))
            .build()
    }

    /// Builds a context whose request URI is `uri`.
    fn cx_with_uri(uri: &str) -> Cx {
        let (parts, ()) = http::Request::builder()
            .uri(uri)
            .body(())
            .expect("a valid request URI")
            .into_parts();

        CxTestBuilder::new().request_context(parts).build()
    }

    /// Assigns `params` to `path` and returns the URL it produces.
    fn assign(path: &str, params: &impl HrefParams) -> String {
        let mut out = String::new();
        params.assign(Path::new(path), &mut out);
        out
    }

    /// Appends `queries` to an empty URL and returns the result.
    fn assign_queries(queries: &impl HrefQueries) -> String {
        let mut out = String::new();
        queries.assign(&mut out);
        out
    }

    #[test]
    fn writes_a_static_path_verbatim() {
        assert_eq!(assign("/users/all", &()), "/users/all");
    }

    #[test]
    fn writes_the_root_path_as_a_slash() {
        assert_eq!(assign("/", &()), "/");
    }

    #[test]
    fn fills_parameters_in_declaration_order() {
        assert_eq!(
            assign(
                "/users/{id}/posts/{post_id}",
                &(Param("id", "42"), Param("post_id", "7")),
            ),
            "/users/42/posts/7"
        );
    }

    #[test]
    fn skips_group_segments() {
        assert_eq!(
            assign("/(auth)/users/{id}", &(Param("id", "42"),)),
            "/users/42"
        );
    }

    #[test]
    fn a_group_only_path_addresses_the_root() {
        assert_eq!(assign("/(marketing)", &()), "/");
    }

    #[test]
    fn fills_a_catch_all_with_one_segment_per_element() {
        assert_eq!(
            assign("/docs/{*rest}", &(Tail("rest", &["guides", "start"]),)),
            "/docs/guides/start"
        );
    }

    #[test]
    fn percent_encodes_a_segment() {
        assert_eq!(
            assign("/users/{id}", &(Param("id", "a/b?c#d e"),)),
            "/users/a%2Fb%3Fc%23d%20e"
        );
    }

    #[test]
    fn percent_encodes_each_segment_of_a_catch_all() {
        assert_eq!(
            assign("/docs/{*rest}", &(Tail("rest", &["a/b", "caf\u{e9}"]),)),
            "/docs/a%2Fb/caf%C3%A9"
        );
    }

    #[test]
    fn leaves_the_unreserved_characters_of_a_segment_alone() {
        assert_eq!(
            assign("/docs/{slug}", &(Param("slug", "getting-started_v1.0~x"),)),
            "/docs/getting-started_v1.0~x"
        );
    }

    #[test]
    #[should_panic(expected = "provided parameter \"user_id\" does not fill path parameter \"id\"")]
    fn rejects_a_parameter_name_mismatch() {
        let _ = assign("/users/{id}", &(Param("user_id", "42"),));
    }

    #[test]
    #[should_panic(
        expected = "parameter \"id\" fills one segment of `/users/{id}`, but was given 2"
    )]
    fn rejects_a_parameter_spanning_several_segments() {
        let _ = assign("/users/{id}", &(Tail("id", &["a", "b"]),));
    }

    #[test]
    #[should_panic(
        expected = "catch-all parameter \"rest\" in `/docs/{*rest}` was given no segment"
    )]
    fn rejects_an_empty_catch_all() {
        let _ = assign("/docs/{*rest}", &(Tail("rest", &[]),));
    }

    #[test]
    #[should_panic(expected = "parameter \"id\" in `/users/{id}` was given an empty segment")]
    fn rejects_an_empty_segment() {
        let _ = assign("/users/{id}", &(Param("id", ""),));
    }

    #[test]
    #[should_panic(expected = "parameter \"id\" in `/users/{id}` was given \"..\"")]
    fn rejects_a_parent_segment() {
        let _ = assign("/users/{id}", &(Param("id", ".."),));
    }

    #[test]
    #[should_panic(expected = "parameter \"rest\" in `/docs/{*rest}` was given \".\"")]
    fn rejects_a_current_segment_inside_a_catch_all() {
        let _ = assign("/docs/{*rest}", &(Tail("rest", &["guides", "."]),));
    }

    #[test]
    fn keeps_a_segment_that_only_starts_with_a_dot() {
        assert_eq!(
            assign("/files/{name}", &(Param("name", ".gitignore"),)),
            "/files/.gitignore"
        );
    }

    #[test]
    #[should_panic(expected = "declares fewer parameters than the href provides")]
    fn rejects_more_parameters_than_the_path_declares() {
        let _ = assign("/users/{id}", &(Param("id", "42"), Param("extra", "1")));
    }

    #[test]
    #[should_panic(expected = "no value provided for path parameter \"id\"")]
    fn rejects_an_unfilled_path_parameter() {
        let _ = assign("/users/{id}", &());
    }

    #[test]
    fn writes_no_query_without_items() {
        assert_eq!(assign_queries(&()), "");
    }

    #[test]
    fn concatenates_query_items() {
        assert_eq!(
            assign_queries(&([("tag", "rust")], [("page", "2"), ("sort", "asc")])),
            "?tag=rust&page=2&sort=asc"
        );
    }

    #[test]
    fn skips_query_items_that_serialize_to_nothing() {
        #[derive(serde::Serialize)]
        struct Empty {}

        assert_eq!(
            assign_queries(&(Empty {}, [("page", "2")], Empty {})),
            "?page=2"
        );
    }

    #[test]
    fn percent_encodes_query_values() {
        assert_eq!(assign_queries(&([("tag", "a b&c")],)), "?tag=a+b%26c");
    }

    #[test]
    fn resolves_a_relative_url_with_query_and_fragment() {
        let url = href("/users/{id}", (Param("id", "42"),))
            .query([("page", "2")])
            .fragment("bio")
            .resolve(&Cx::default());

        assert_eq!(url, "/users/42?page=2#bio");
    }

    #[test]
    fn the_macro_takes_the_parameters_as_a_list() {
        let url = href!(
            "/users/{id}/posts/{post_id}",
            Param("id", "42"),
            Param("post_id", "7")
        )
        .resolve(&Cx::default());

        assert_eq!(url, "/users/42/posts/7");
    }

    #[test]
    fn the_macro_without_parameters_resolves_a_static_path() {
        assert_eq!(href!("/users").resolve(&Cx::default()), "/users");
    }

    #[test]
    fn a_set_form_overrides_the_context_form() {
        let cx = Cx::default().with(UrlForm::Absolute);
        let url = href("/users", ()).relative().resolve(&cx);

        assert_eq!(url, "/users");
    }

    #[test]
    fn resolves_an_absolute_url_behind_the_base_url() {
        let url = href("/users/{id}", (Param("id", "42"),))
            .query([("page", "2")])
            .fragment("bio")
            .absolute()
            .resolve(&cx_with_base_url());

        assert_eq!(url, "https://example.com/users/42?page=2#bio");
    }

    #[test]
    fn an_absolute_root_url_keeps_its_slash() {
        assert_eq!(
            href("/", ()).absolute().resolve(&cx_with_base_url()),
            "https://example.com/"
        );
    }

    #[test]
    fn the_context_form_turns_a_url_absolute() {
        let cx = cx_with_base_url().with(UrlForm::Absolute);

        assert_eq!(href("/users", ()).resolve(&cx), "https://example.com/users");
    }

    #[test]
    fn is_current_on_the_request_path() {
        assert!(href("/users", ()).is_current(&cx_with_uri("/users")));
        assert!(!href("/users", ()).is_current(&cx_with_uri("/users/42")));
    }

    #[test]
    fn is_current_fills_in_the_parameters() {
        let href = href("/users/{id}", (Param("id", "42"),));

        assert!(href.is_current(&cx_with_uri("/users/42")));
        assert!(!href.is_current(&cx_with_uri("/users/7")));
    }

    #[test]
    fn is_current_compares_the_encoded_path() {
        let href = href("/users/{id}", (Param("id", "a/b"),));

        assert!(href.is_current(&cx_with_uri("/users/a%2Fb")));
        assert!(!href.is_current(&cx_with_uri("/users/a/b")));
    }

    #[test]
    fn the_root_href_is_current_on_the_root() {
        assert!(href("/", ()).is_current(&cx_with_uri("/")));
    }

    #[test]
    fn an_unspecified_query_is_left_out_of_the_comparison() {
        assert!(href("/users", ()).is_current(&cx_with_uri("/users?page=2")));
    }

    #[test]
    fn a_specified_query_must_match_the_request_query() {
        let href = href("/users", ()).query([("page", "2")]);

        assert!(href.is_current(&cx_with_uri("/users?page=2")));
        assert!(!href.is_current(&cx_with_uri("/users")));
        assert!(!href.is_current(&cx_with_uri("/users?page=3")));
        assert!(!href.is_current(&cx_with_uri("/users?page=2&sort=asc")));
    }

    #[test]
    fn the_query_comparison_ignores_order_and_encoding() {
        let href = href("/users", ()).query([("page", "2"), ("sort", "a b")]);

        assert!(href.is_current(&cx_with_uri("/users?sort=a%20b&page=2")));
    }

    #[test]
    fn a_query_that_serializes_to_nothing_demands_a_bare_request() {
        #[derive(serde::Serialize)]
        struct Empty {}

        let href = href("/users", ()).query(Empty {});

        assert!(href.is_current(&cx_with_uri("/users")));
        assert!(!href.is_current(&cx_with_uri("/users?page=2")));
    }

    #[test]
    fn the_fragment_is_left_out_of_the_comparison() {
        let href = href("/users", ()).fragment("bio");

        assert!(href.is_current(&cx_with_uri("/users")));
    }

    #[test]
    fn builder_appends_queries_and_replaces_the_fragment() {
        let href = href("/users", ())
            .query([("a", "1")])
            .fragment("one")
            .query([("b", "2")])
            .fragment("two");

        assert_eq!(assign_queries(&href.queries), "?a=1&b=2");
        assert_eq!(href.fragment, Some("two"));
    }
}
