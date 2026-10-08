use std::hash::{DefaultHasher, Hash, Hasher};

use proc_macro2::Span;
use syn::{
    Ident, LitStr,
    parse::{Parse, ParseStream},
};
use topcoat_core_grammar::ParseOption;
use topcoat_router::{Path, PathSegment};

/// An absolute route path for a procedure or shard endpoint.
///
/// Accepts group segments but rejects dynamic and catch-all parameters.
pub struct EndpointPath {
    pub lit: LitStr,
}

impl EndpointPath {
    /// Returns the supplied path, or generates a path under `prefix` for the
    /// function named `ident`.
    ///
    /// A generated path is a hash of the function's name and source location,
    /// so it stays the same until the function is renamed or moved.
    #[must_use]
    pub fn resolve(path: Option<&Self>, prefix: &str, ident: &Ident) -> LitStr {
        path.map_or_else(
            || {
                let span = ident.span();
                let start = span.start();
                let mut hasher = DefaultHasher::new();
                (span.file(), start.line, start.column, ident.to_string()).hash(&mut hasher);
                LitStr::new(
                    &format!("{prefix}/{:016x}", hasher.finish()),
                    Span::call_site(),
                )
            },
            |path| path.lit.clone(),
        )
    }

    /// Converts a route path to its request URL, removing group segments.
    /// Root and group-only paths produce `/`.
    ///
    /// # Panics
    ///
    /// Panics if `path` is not a valid route path.
    #[must_use]
    pub fn url(path: &LitStr) -> LitStr {
        LitStr::new(&Path::new(&path.value()).to_matchit_path(), path.span())
    }
}

impl Parse for EndpointPath {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lit: LitStr = input.parse()?;
        let value = lit.value();
        if value.starts_with('.') {
            return Err(syn::Error::new(
                lit.span(),
                "relative paths are not supported here; \
                 use an absolute path starting with `/`",
            ));
        }
        let path = Path::from_str(&value).map_err(|err| syn::Error::new(lit.span(), err))?;
        if path
            .segments()
            .any(|segment| matches!(segment, PathSegment::Param(_) | PathSegment::CatchAll(_)))
        {
            return Err(syn::Error::new(
                lit.span(),
                "the path cannot declare parameters; arguments travel in the request body",
            ));
        }
        Ok(Self { lit })
    }
}

impl ParseOption for EndpointPath {
    fn peek(input: ParseStream) -> bool {
        input.peek(LitStr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_err(source: &str) -> String {
        match syn::parse_str::<EndpointPath>(source) {
            Ok(_) => panic!("expected parse error for `{source}`"),
            Err(err) => err.to_string(),
        }
    }

    #[test]
    fn accepts_an_absolute_path_with_a_group_segment() {
        let path: EndpointPath = syn::parse_str(r#""/(api)/search""#).unwrap();
        assert_eq!(path.lit.value(), "/(api)/search");
    }

    #[test]
    fn rejects_a_relative_path() {
        assert!(parse_err(r#""./search""#).contains("absolute"));
    }

    #[test]
    fn rejects_a_path_without_a_leading_slash() {
        assert!(parse_err(r#""search""#).contains("invalid path"));
    }

    #[test]
    fn rejects_parameter_and_catch_all_segments() {
        assert!(parse_err(r#""/items/{id}""#).contains("parameters"));
        assert!(parse_err(r#""/files/{*rest}""#).contains("parameters"));
    }

    /// Resolves a generated path for an identifier parsed from `source`, so
    /// that leading whitespace and newlines move its source location.
    fn generated(source: &str) -> String {
        let ident: Ident = syn::parse_str(source).unwrap();
        EndpointPath::resolve(None, "/prefix", &ident).value()
    }

    #[test]
    fn a_named_path_is_served_verbatim() {
        let path: EndpointPath = syn::parse_str(r#""/search""#).unwrap();
        let ident: Ident = syn::parse_str("search").unwrap();
        assert_eq!(
            EndpointPath::resolve(Some(&path), "/unused", &ident).value(),
            "/search"
        );
    }

    #[test]
    fn a_missing_path_falls_below_the_prefix() {
        let path = generated("save");
        let rest = path.strip_prefix("/prefix/").expect(&path);
        assert!(!rest.is_empty(), "{path}");
        assert!(rest.chars().all(|c| c.is_ascii_hexdigit()), "{path}");
    }

    #[test]
    fn generated_paths_differ_by_name() {
        assert_ne!(generated("save"), generated("load"));
    }

    #[test]
    fn generated_paths_differ_by_location() {
        assert_ne!(generated("save"), generated("  save"));
        assert_ne!(generated("save"), generated("\nsave"));
    }

    #[test]
    fn the_url_strips_group_segments_and_keeps_the_root_addressable() {
        let url = |path: &str| EndpointPath::url(&LitStr::new(path, Span::call_site())).value();
        assert_eq!(url("/search"), "/search");
        assert_eq!(url("/(api)/search"), "/search");
        assert_eq!(url("/(api)"), "/");
        assert_eq!(url("/"), "/");
    }
}
