use std::{
    borrow::Cow,
    fmt::{self, Display, Write},
    marker::PhantomData,
};

use percent_encoding::{AsciiSet, CONTROLS, utf8_percent_encode};
use serde::Serialize;
use topcoat_core::{
    base_url::base_url,
    context::Cx,
    url_form::{UrlForm, url_form},
};
use topcoat_view::{AttributeValueViewParts, NodeViewParts, PartsWriter};

use crate::{CatchAllSegments, Path, PathSegment, request::uri};

/// The destination an `href!` points at, resolved to the route [`Path`] the
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

/// A path parameter that accepts values of type `V` in an href.
///
/// `path_param!` implements this trait for the marker type it declares, which
/// is named after the parameter. `href!` uses it to turn a `param: value`
/// argument into URL segments.
pub trait HrefParam<V> {
    /// The parameter's name in the route path.
    const NAME: &'static str;

    /// Pushes the segments `value` fills the parameter with: one for a regular
    /// parameter, one per element for a catch-all.
    fn segments(value: &V, segments: &mut HrefSegments<'_>);

    /// Wraps `value` as an argument of an href.
    fn arg(value: V) -> HrefArg<Self, V>
    where
        Self: Sized,
    {
        HrefArg {
            value,
            param: PhantomData,
        }
    }
}

/// A value passed to `href!` for the path parameter `P`.
///
/// The value is kept as passed and written into the URL each time the href
/// renders.
pub struct HrefArg<P, V> {
    value: V,
    param: PhantomData<fn() -> P>,
}

impl<P, V> HrefArg<P, V>
where
    P: HrefParam<V>,
{
    /// Writes the value into `out` as the parameter `name` of `path`,
    /// checking that it filled as much of the path as the parameter stands
    /// for.
    ///
    /// # Panics
    ///
    /// Panics if a regular parameter is not given exactly one segment, or a
    /// catch-all none: a catch-all matches at least one segment, so an empty
    /// one addresses a path its route does not serve.
    fn fill(&self, name: &str, catch_all: bool, path: &Path, out: &mut String) {
        let mut segments = HrefSegments {
            out,
            name,
            path,
            count: 0,
        };
        P::segments(&self.value, &mut segments);
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
}

/// The URL segments a path parameter is filled with.
///
/// [`push`](Self::push) appends one segment. The separating `/` and the
/// escaping are written here, so a value never spells out either itself.
pub struct HrefSegments<'out> {
    out: &'out mut String,
    name: &'out str,
    path: &'out Path,
    count: usize,
}

impl HrefSegments<'_> {
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
    pub fn push(&mut self, segment: &(impl Display + ?Sized)) {
        let (name, path) = (self.name, self.path);
        let start = self.out.len();
        self.out.push('/');
        write!(PercentEncoded(self.out), "{segment}").unwrap();

        let rendered = &self.out[start + 1..];
        assert!(
            !rendered.is_empty(),
            "parameter \"{name}\" in `{path}` was given an empty segment"
        );
        assert!(
            !matches!(rendered, "." | ".."),
            "parameter \"{name}\" in `{path}` was given \"{rendered}\", which \
             addresses another path instead of filling a segment"
        );
        self.count += 1;
    }
}

/// A list of string segments: the value of a catch-all path parameter
/// declared without a type.
///
/// It is implemented for slices, arrays, and vectors of strings, for
/// references to them, and for the [`CatchAllSegments`] read from a request.
pub trait SegmentList {
    /// Calls `f` with each segment, in order.
    fn for_each_segment(&self, f: impl FnMut(&str));
}

impl<S> SegmentList for [S]
where
    S: AsRef<str>,
{
    fn for_each_segment(&self, mut f: impl FnMut(&str)) {
        for segment in self {
            f(segment.as_ref());
        }
    }
}

impl<S, const N: usize> SegmentList for [S; N]
where
    S: AsRef<str>,
{
    fn for_each_segment(&self, f: impl FnMut(&str)) {
        self.as_slice().for_each_segment(f);
    }
}

impl<S> SegmentList for Vec<S>
where
    S: AsRef<str>,
{
    fn for_each_segment(&self, f: impl FnMut(&str)) {
        self.as_slice().for_each_segment(f);
    }
}

impl<T> SegmentList for &T
where
    T: SegmentList + ?Sized,
{
    fn for_each_segment(&self, f: impl FnMut(&str)) {
        (**self).for_each_segment(f);
    }
}

impl SegmentList for CatchAllSegments<'_> {
    fn for_each_segment(&self, f: impl FnMut(&str)) {
        self.clone().for_each(f);
    }
}

/// The path parameters of an href: a tuple of up to eight [`HrefArg`]s, one
/// for each parameter of the path, in any order.
pub trait HrefParams {
    /// Writes `path` into `out`, filling each parameter with the argument of
    /// the same name.
    ///
    /// # Panics
    ///
    /// Panics if the arguments do not line up with the path's parameters: a
    /// parameter no argument fills, an argument the path declares no parameter
    /// for, a parameter given two arguments, or an argument that fills its
    /// parameter with the wrong number of segments. Panics as well if a
    /// segment renders to nothing, `.`, or `..`.
    fn assign(&self, path: &Path, out: &mut String);
}

/// Generates the `HrefParams` impl for a tuple of arguments, each given as its
/// parameter type and value type.
///
/// Each argument is bound to a variable named like its parameter type. The
/// variable lives in the value namespace and the type in the type namespace,
/// so both stay reachable.
macro_rules! impl_href_params_tuples {
    ( $(($param:ident, $value:ident)),* ) => {
        #[allow(non_snake_case, unused_mut, unused_variables)]
        impl<$($param, $value,)*> HrefParams for ($(HrefArg<$param, $value>,)*)
        where
            $($param: HrefParam<$value>,)*
        {
            fn assign(&self, path: &Path, out: &mut String) {
                let ($($param,)*) = self;
                let start = out.len();
                let mut filled = 0;

                for segment in path.segments() {
                    let (name, catch_all) = match segment {
                        PathSegment::Group(_) => continue,
                        PathSegment::Static(segment) => {
                            out.push('/');
                            out.push_str(segment);
                            continue;
                        }
                        PathSegment::Param(name) => (name, false),
                        PathSegment::CatchAll(name) => (name, true),
                    };
                    $(
                        if <$param as HrefParam<$value>>::NAME == name {
                            $param.fill(name, catch_all, path, out);
                            filled += 1;
                            continue;
                        }
                    )*
                    panic!("no value provided for path parameter \"{name}\" in `{path}`");
                }

                let names: &[&str] = &[$(<$param as HrefParam<$value>>::NAME,)*];
                if filled != names.len() {
                    reject_leftover_args(names, path);
                }

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
impl_href_params_tuples!((P1, V1));
impl_href_params_tuples!((P1, V1), (P2, V2));
impl_href_params_tuples!((P1, V1), (P2, V2), (P3, V3));
impl_href_params_tuples!((P1, V1), (P2, V2), (P3, V3), (P4, V4));
impl_href_params_tuples!((P1, V1), (P2, V2), (P3, V3), (P4, V4), (P5, V5));
impl_href_params_tuples!((P1, V1), (P2, V2), (P3, V3), (P4, V4), (P5, V5), (P6, V6));
impl_href_params_tuples!(
    (P1, V1),
    (P2, V2),
    (P3, V3),
    (P4, V4),
    (P5, V5),
    (P6, V6),
    (P7, V7)
);
impl_href_params_tuples!(
    (P1, V1),
    (P2, V2),
    (P3, V3),
    (P4, V4),
    (P5, V5),
    (P6, V6),
    (P7, V7),
    (P8, V8)
);

/// Panics on the arguments of an href that filled no parameter of `path`.
///
/// Every parameter takes the first argument of its name, so an argument left
/// over names a parameter the path does not declare, or repeats one.
fn reject_leftover_args(names: &[&str], path: &Path) {
    for (index, &name) in names.iter().enumerate() {
        assert!(
            path.segments()
                .any(|segment| segment.param_name() == Some(name)),
            "`{path}` declares no parameter \"{name}\""
        );
        assert!(
            !names[..index].contains(&name),
            "path parameter \"{name}\" in `{path}` was given more than once"
        );
    }
}

/// The query of an href: a tuple of up to eight [`Serialize`] items,
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

/// A URL being built by `href!`.
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

impl<T, P> Href<T, P, (), &'static str> {
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
/// to, e.g. `href=(href!(...))`.
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
    use topcoat_core::{base_url::BaseUrl, context::Cx};

    use super::*;

    /// Declares a parameter named `$name` that fills one segment with a
    /// string.
    macro_rules! segment_param {
        ($ty:ident = $name:literal) => {
            struct $ty;

            impl<V: AsRef<str>> HrefParam<V> for $ty {
                const NAME: &'static str = $name;

                fn segments(value: &V, segments: &mut HrefSegments<'_>) {
                    segments.push(value.as_ref());
                }
            }
        };
    }

    /// Declares a parameter named `$name` that fills one segment per element
    /// of a list.
    macro_rules! catch_all_param {
        ($ty:ident = $name:literal) => {
            struct $ty;

            impl<V: SegmentList> HrefParam<V> for $ty {
                const NAME: &'static str = $name;

                fn segments(value: &V, segments: &mut HrefSegments<'_>) {
                    value.for_each_segment(|segment| segments.push(segment));
                }
            }
        };
    }

    segment_param!(Id = "id");
    segment_param!(PostId = "post_id");
    segment_param!(UserId = "user_id");
    segment_param!(Name = "name");
    segment_param!(Slug = "slug");
    segment_param!(Extra = "extra");
    catch_all_param!(Rest = "rest");
    catch_all_param!(IdList = "id");

    /// Builds a context carrying `https://example.com` as its base URL.
    fn cx_with_base_url() -> Cx {
        Cx::new().with(BaseUrl::new("https://example.com").expect("a valid base URL"))
    }

    /// Builds a context whose request URI is `uri`.
    fn cx_with_uri(uri: &str) -> Cx {
        let (parts, ()) = http::Request::builder()
            .uri(uri)
            .body(())
            .expect("a valid request URI")
            .into_parts();

        Cx::new().with(parts)
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
    fn fills_parameters_by_name_in_any_order() {
        assert_eq!(
            assign(
                "/users/{id}/posts/{post_id}",
                &(PostId::arg("7"), Id::arg("42")),
            ),
            "/users/42/posts/7"
        );
    }

    #[test]
    fn skips_group_segments() {
        assert_eq!(
            assign("/(auth)/users/{id}", &(Id::arg("42"),)),
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
            assign("/docs/{*rest}", &(Rest::arg(["guides", "start"]),)),
            "/docs/guides/start"
        );
    }

    #[test]
    fn a_catch_all_accepts_owned_and_borrowed_lists() {
        let owned = vec!["guides".to_owned(), "start".to_owned()];

        assert_eq!(
            assign("/docs/{*rest}", &(Rest::arg(&owned),)),
            "/docs/guides/start"
        );
        assert_eq!(
            assign("/docs/{*rest}", &(Rest::arg(owned.as_slice()),)),
            "/docs/guides/start"
        );
        assert_eq!(
            assign("/docs/{*rest}", &(Rest::arg(owned),)),
            "/docs/guides/start"
        );
    }

    #[test]
    fn renders_the_same_url_each_time() {
        let href = Href::new("/docs/{*rest}", (Rest::arg(vec!["a", "b"]),));
        let cx = Cx::new();

        assert_eq!(href.resolve(&cx), "/docs/a/b");
        assert_eq!(href.resolve(&cx), "/docs/a/b");
    }

    #[test]
    fn percent_encodes_a_segment() {
        assert_eq!(
            assign("/users/{id}", &(Id::arg("a/b?c#d e"),)),
            "/users/a%2Fb%3Fc%23d%20e"
        );
    }

    #[test]
    fn percent_encodes_each_segment_of_a_catch_all() {
        assert_eq!(
            assign("/docs/{*rest}", &(Rest::arg(["a/b", "caf\u{e9}"]),)),
            "/docs/a%2Fb/caf%C3%A9"
        );
    }

    #[test]
    fn leaves_the_unreserved_characters_of_a_segment_alone() {
        assert_eq!(
            assign("/docs/{slug}", &(Slug::arg("getting-started_v1.0~x"),)),
            "/docs/getting-started_v1.0~x"
        );
    }

    #[test]
    #[should_panic(expected = "no value provided for path parameter \"id\"")]
    fn rejects_a_value_named_after_another_parameter() {
        let _ = assign("/users/{id}", &(UserId::arg("42"),));
    }

    #[test]
    #[should_panic(expected = "parameter \"id\" fills one segment")]
    fn rejects_a_parameter_spanning_several_segments() {
        let _ = assign("/users/{id}", &(IdList::arg(["a", "b"]),));
    }

    #[test]
    #[should_panic(expected = "catch-all parameter \"rest\"")]
    fn rejects_an_empty_catch_all() {
        let _ = assign("/docs/{*rest}", &(Rest::arg(Vec::<&str>::new()),));
    }

    #[test]
    #[should_panic(expected = "parameter \"id\" in `/users/{id}` was given an empty segment")]
    fn rejects_an_empty_segment() {
        let _ = assign("/users/{id}", &(Id::arg(""),));
    }

    #[test]
    #[should_panic(expected = "parameter \"id\" in `/users/{id}` was given \"..\"")]
    fn rejects_a_parent_segment() {
        let _ = assign("/users/{id}", &(Id::arg(".."),));
    }

    #[test]
    #[should_panic(expected = "parameter \"rest\" in `/docs/{*rest}` was given \".\"")]
    fn rejects_a_current_segment_inside_a_catch_all() {
        let _ = assign("/docs/{*rest}", &(Rest::arg(["guides", "."]),));
    }

    #[test]
    fn keeps_a_segment_that_only_starts_with_a_dot() {
        assert_eq!(
            assign("/files/{name}", &(Name::arg(".gitignore"),)),
            "/files/.gitignore"
        );
    }

    #[test]
    #[should_panic(expected = "declares no parameter \"extra\"")]
    fn rejects_more_parameters_than_the_path_declares() {
        let _ = assign("/users/{id}", &(Id::arg("42"), Extra::arg("1")));
    }

    #[test]
    #[should_panic(expected = "\"id\" in `/users/{id}` was given more than once")]
    fn rejects_a_parameter_given_twice() {
        let _ = assign("/users/{id}", &(Id::arg("42"), Id::arg("7")));
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
        let url = Href::new("/users/{id}", (Id::arg("42"),))
            .query([("page", "2")])
            .fragment("bio")
            .resolve(&Cx::new());

        assert_eq!(url, "/users/42?page=2#bio");
    }

    #[test]
    fn a_set_form_overrides_the_context_form() {
        let cx = Cx::new().with(UrlForm::Absolute);
        let url = Href::new("/users", ()).relative().resolve(&cx);

        assert_eq!(url, "/users");
    }

    #[test]
    fn resolves_an_absolute_url_behind_the_base_url() {
        let url = Href::new("/users/{id}", (Id::arg("42"),))
            .query([("page", "2")])
            .fragment("bio")
            .absolute()
            .resolve(&cx_with_base_url());

        assert_eq!(url, "https://example.com/users/42?page=2#bio");
    }

    #[test]
    fn an_absolute_root_url_keeps_its_slash() {
        assert_eq!(
            Href::new("/", ()).absolute().resolve(&cx_with_base_url()),
            "https://example.com/"
        );
    }

    #[test]
    fn the_context_form_turns_a_url_absolute() {
        let cx = cx_with_base_url().with(UrlForm::Absolute);

        assert_eq!(
            Href::new("/users", ()).resolve(&cx),
            "https://example.com/users"
        );
    }

    #[test]
    fn is_current_on_the_request_path() {
        assert!(Href::new("/users", ()).is_current(&cx_with_uri("/users")));
        assert!(!Href::new("/users", ()).is_current(&cx_with_uri("/users/42")));
    }

    #[test]
    fn is_current_fills_in_the_parameters() {
        let href = Href::new("/users/{id}", (Id::arg("42"),));

        assert!(href.is_current(&cx_with_uri("/users/42")));
        assert!(!href.is_current(&cx_with_uri("/users/7")));
    }

    #[test]
    fn is_current_compares_the_encoded_path() {
        let href = Href::new("/users/{id}", (Id::arg("a/b"),));

        assert!(href.is_current(&cx_with_uri("/users/a%2Fb")));
        assert!(!href.is_current(&cx_with_uri("/users/a/b")));
    }

    #[test]
    fn the_root_href_is_current_on_the_root() {
        assert!(Href::new("/", ()).is_current(&cx_with_uri("/")));
    }

    #[test]
    fn an_unspecified_query_is_left_out_of_the_comparison() {
        assert!(Href::new("/users", ()).is_current(&cx_with_uri("/users?page=2")));
    }

    #[test]
    fn a_specified_query_must_match_the_request_query() {
        let href = Href::new("/users", ()).query([("page", "2")]);

        assert!(href.is_current(&cx_with_uri("/users?page=2")));
        assert!(!href.is_current(&cx_with_uri("/users")));
        assert!(!href.is_current(&cx_with_uri("/users?page=3")));
        assert!(!href.is_current(&cx_with_uri("/users?page=2&sort=asc")));
    }

    #[test]
    fn the_query_comparison_ignores_order_and_encoding() {
        let href = Href::new("/users", ()).query([("page", "2"), ("sort", "a b")]);

        assert!(href.is_current(&cx_with_uri("/users?sort=a%20b&page=2")));
    }

    #[test]
    fn a_query_that_serializes_to_nothing_demands_a_bare_request() {
        #[derive(serde::Serialize)]
        struct Empty {}

        let href = Href::new("/users", ()).query(Empty {});

        assert!(href.is_current(&cx_with_uri("/users")));
        assert!(!href.is_current(&cx_with_uri("/users?page=2")));
    }

    #[test]
    fn the_fragment_is_left_out_of_the_comparison() {
        let href = Href::new("/users", ()).fragment("bio");

        assert!(href.is_current(&cx_with_uri("/users")));
    }

    #[test]
    fn builder_appends_queries_and_replaces_the_fragment() {
        let href = Href::new("/users", ())
            .query([("a", "1")])
            .fragment("one")
            .query([("b", "2")])
            .fragment("two");

        assert_eq!(assign_queries(&href.queries), "?a=1&b=2");
        assert_eq!(href.fragment, Some("two"));
    }
}
