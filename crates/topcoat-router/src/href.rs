use std::fmt::{Display, Write};

use serde::Serialize;
use topcoat_core::{
    base_url::base_url,
    context::Cx,
    url_form::{UrlForm, url_form},
};
use topcoat_view::{AttributeValueViewParts, NodeViewParts, PartsWriter};

use crate::{Path, PathSegment, PathSegments};

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
/// An [`href`] writes each provided value into the path segment whose
/// parameter carries the same name.
pub trait HrefParam {
    /// What the parameter renders as in the URL: one segment for a regular
    /// parameter, the `/`-joined segments for a catch-all.
    type Value: Display + ?Sized;

    /// The name of the path parameter this value fills.
    fn name(&self) -> &str;

    /// The value written into the URL in the parameter's place.
    fn value(&self) -> &Self::Value;
}

impl<T> HrefParam for &T
where
    T: HrefParam,
{
    type Value = T::Value;

    fn name(&self) -> &str {
        (*self).name()
    }

    fn value(&self) -> &Self::Value {
        (*self).value()
    }
}

/// The path parameters of an [`href`]: a tuple of up to eight [`HrefParam`]
/// values, in the order the path declares its parameters.
pub trait HrefParams {
    /// Writes `path` into `out` with every parameter filled in.
    ///
    /// # Panics
    ///
    /// Panics if the values do not line up with the path's parameters: a name
    /// mismatch, a value the path declares no parameter for, or a parameter
    /// no value fills.
    fn assign(&self, path: &Path, out: &mut String);
}

/// Generates the `HrefParams` impl for a tuple of parameters `P1..Pn`.
///
/// The parameters fill the path's capturing segments in order: the walk copies
/// static segments into the output, skips group segments, and writes each
/// parameter's value where the path declares the parameter of the same name.
macro_rules! impl_href_params_tuples {
    ( $($ty:ident),* ) => {
        #[allow(non_snake_case, unused_mut, unused_variables)]
        impl<$($ty,)*> HrefParams for ($($ty,)*)
        where
            $($ty: HrefParam,)*
        {
            fn assign(&self, path: &Path, out: &mut String) {
                let start = out.len();
                let ($($ty,)*) = self;
                let mut segments = path.segments();
                $(
                    let name = next_param_name(&mut segments, path, out);
                    assert_eq!(
                        name,
                        $ty.name(),
                        "provided parameter \"{}\" does not fill path parameter \
                         \"{name}\" in `{path}`",
                        $ty.name(),
                    );
                    write!(out, "/{}", $ty.value()).unwrap();
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
/// captures under, copying the static segments crossed on the way into `out`.
///
/// # Panics
///
/// Panics if the path declares no further capturing segment.
fn next_param_name<'path>(
    segments: &mut PathSegments<'path>,
    path: &Path,
    out: &mut String,
) -> &'path str {
    loop {
        match segments.next() {
            Some(PathSegment::Group(_)) => {}
            Some(PathSegment::Static(segment)) => {
                out.push('/');
                out.push_str(segment);
            }
            Some(PathSegment::Param(name) | PathSegment::CatchAll(name)) => return name,
            None => panic!("`{path}` declares fewer parameters than the href provides"),
        }
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
    /// Appends the items, serialized and concatenated into one query string,
    /// to the URL in `out`.
    ///
    /// # Panics
    ///
    /// Panics if an item does not serialize to a URL query string.
    fn assign(&self, out: &mut String);
}

/// Generates the `HrefQueries` impl for a tuple of query items `Q1..Qn`.
///
/// Each item serializes to a query string on its own; the non-empty results
/// are concatenated, the first behind `?` and the rest behind `&`.
macro_rules! impl_href_queries_tuples {
    ( $($ty:ident),* ) => {
        #[allow(non_snake_case, unused_assignments, unused_mut, unused_variables)]
        impl<$($ty,)*> HrefQueries for ($($ty,)*)
        where
            $($ty: Serialize,)*
        {
            fn assign(&self, out: &mut String) {
                let ($($ty,)*) = self;
                let mut separator = '?';
                $(
                    if write_query($ty, separator, out) {
                        separator = '&';
                    }
                )*
            }
        }
    };
}

impl_href_queries_tuples!();
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

/// Turns a page or route into an URL string.
///
/// The first parameter, `target`, is the route handler the URL should be
/// pointing to. The URL is built from the path the handler is mounted at, so
/// it stays in sync when the route moves. A [`Path`] or a plain path string
/// works as well.
///
/// Inside a handler's own body its name refers to the handler function, so a
/// handler linking to itself names its marker as a type, e.g. `posts {}`, or
/// uses the [`href!`] macro, which does that on its own.
///
/// `params` fills in the path's parameters: one `path_param!` value per
/// parameter, in the order the path declares them, passed as a tuple. A path
/// without parameters takes `()`.
///
/// The result can be extended with a [`query`](Href::query) string, a
/// [`fragment`](Href::fragment), and an [`absolute`](Href::absolute) or
/// [`relative`](Href::relative) form. Use it directly in a view to render
/// the URL, or call [`resolve`](Href::resolve) to get the string, e.g. for
/// a redirect.
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
/// The [`href!`] macro builds the same URL with the parameters listed as
/// plain arguments instead of a tuple.
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
/// a link to a post reads as `href!(post, PostId(post.id))`.
///
/// The result can be extended with a [`query`](Href::query) string, a
/// [`fragment`](Href::fragment), and an [`absolute`](Href::absolute) or
/// [`relative`](Href::relative) form. Use it directly in a view to render
/// the URL, or call [`resolve`](Href::resolve) to get the string, e.g. for
/// a redirect.
///
/// ```
/// use serde::Serialize;
/// use topcoat::{
///     Result,
///     context::Cx,
///     router::{
///         error::{SeeOther, see_other},
///         href, page, path_param, route,
///     },
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
///     Ok(see_other(&href!(post, PostId(*post_id)).resolve(cx)))
/// }
/// # fn main() {}
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
    pub fn resolve(self, cx: &Cx) -> String {
        let mut buf = String::new();
        match self.url_form.unwrap_or_else(|| url_form(cx)) {
            UrlForm::Absolute => buf += base_url(cx).as_str(),
            UrlForm::Relative => {}
        }

        self.params.assign(self.target.path(cx), &mut buf);
        self.queries.assign(&mut buf);

        if let Some(fragment) = self.fragment {
            write!(buf, "#{fragment}").unwrap();
        }

        buf
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
    use super::*;

    /// An href parameter with a fixed name and value.
    struct Param(&'static str, &'static str);

    impl HrefParam for Param {
        type Value = str;

        fn name(&self) -> &str {
            self.0
        }

        fn value(&self) -> &str {
            self.1
        }
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
    fn fills_a_catch_all_with_its_joined_segments() {
        assert_eq!(
            assign("/docs/{*rest}", &(Param("rest", "guides/start"),)),
            "/docs/guides/start"
        );
    }

    #[test]
    #[should_panic(expected = "provided parameter \"user_id\" does not fill path parameter \"id\"")]
    fn rejects_a_parameter_name_mismatch() {
        let _ = assign("/users/{id}", &(Param("user_id", "42"),));
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
