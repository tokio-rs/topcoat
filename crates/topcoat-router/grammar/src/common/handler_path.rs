use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    LitStr,
    parse::{Parse, ParseStream},
};
use topcoat_core_grammar::{ParseOption, paths::topcoat_router};

/// The path literal a handler attribute accepts, such as the `"/about"` in
/// `#[page("/about")]`.
///
/// An absolute literal starts with `/` and is the path the handler is served
/// at. A relative literal starts with `./` and names a path below the
/// handler's module path, which the module router joins onto it.
pub struct HandlerPath {
    pub lit: LitStr,
}

impl HandlerPath {
    /// The prefix marking a relative path.
    const RELATIVE_PREFIX: &str = "./";

    /// The path the handler is served at, when the literal is absolute.
    #[must_use]
    pub fn absolute(&self) -> Option<&LitStr> {
        (!self.is_relative()).then_some(&self.lit)
    }

    /// The path below the module path, when the literal is relative.
    ///
    /// The returned literal keeps the span of the written one, so an error in
    /// the path points at the source.
    #[must_use]
    pub fn relative(&self) -> Option<LitStr> {
        let value = self.lit.value();
        // Dropping the leading `.` of `./export` leaves `/export`, the path
        // the module router joins onto the module path.
        value
            .starts_with(Self::RELATIVE_PREFIX)
            .then(|| LitStr::new(&value[1..], self.lit.span()))
    }

    /// The `relative_path` method of a module trait impl, when the literal is
    /// relative.
    #[must_use]
    pub fn relative_path_method(&self) -> Option<TokenStream> {
        let path = self.relative()?;
        Some(quote! {
            fn relative_path(&self) -> &#topcoat_router::Path {
                const PATH: &#topcoat_router::Path = #topcoat_router::Path::new(#path);
                PATH
            }
        })
    }

    fn is_relative(&self) -> bool {
        self.lit.value().starts_with(Self::RELATIVE_PREFIX)
    }
}

impl Parse for HandlerPath {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lit: LitStr = input.parse()?;
        let value = lit.value();
        if value == "." || value == Self::RELATIVE_PREFIX {
            return Err(syn::Error::new(
                lit.span(),
                "a relative path must name a path below the module path; \
                 drop the path to serve the module path itself",
            ));
        }
        if value.starts_with("..") {
            return Err(syn::Error::new(
                lit.span(),
                "a relative path cannot leave the module path; \
                 use an absolute path starting with `/` instead",
            ));
        }
        Ok(Self { lit })
    }
}

impl ParseOption for HandlerPath {
    fn peek(input: ParseStream) -> bool {
        input.peek(LitStr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> HandlerPath {
        syn::parse_str(source).unwrap()
    }

    fn parse_err(source: &str) -> String {
        match syn::parse_str::<HandlerPath>(source) {
            Ok(_) => panic!("expected parse error for `{source}`"),
            Err(err) => err.to_string(),
        }
    }

    #[test]
    fn absolute_path_is_kept_verbatim() {
        let path = parse(r#""/users/{id}""#);
        assert_eq!(path.absolute().unwrap().value(), "/users/{id}");
        assert!(path.relative().is_none());
        assert!(path.relative_path_method().is_none());
    }

    #[test]
    fn relative_path_drops_the_dot() {
        let path = parse(r#""./export""#);
        assert!(path.absolute().is_none());
        assert_eq!(path.relative().unwrap().value(), "/export");
        assert!(path.relative_path_method().is_some());
    }

    #[test]
    fn relative_path_keeps_the_rest_intact() {
        let path = parse(r#""./{id}/(admin)/edit""#);
        assert_eq!(path.relative().unwrap().value(), "/{id}/(admin)/edit");
    }

    #[test]
    fn path_without_leading_slash_or_dot_is_treated_as_absolute() {
        // `Path::new` rejects it at compile time in the expansion; the node
        // does not second-guess that.
        let path = parse(r#""export""#);
        assert_eq!(path.absolute().unwrap().value(), "export");
    }

    #[test]
    fn rejects_the_module_path_itself() {
        assert!(parse_err(r#"".""#).contains("drop the path"));
        assert!(parse_err(r#""./""#).contains("drop the path"));
    }

    #[test]
    fn rejects_parent_paths() {
        assert!(parse_err(r#""../sibling""#).contains("cannot leave the module path"));
        assert!(parse_err(r#""..""#).contains("cannot leave the module path"));
    }
}
