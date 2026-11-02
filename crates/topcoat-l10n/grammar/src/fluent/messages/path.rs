mod resolve;

pub use resolve::*;
use syn::{
    LitStr,
    parse::{Parse, ParseStream},
};
use topcoat_core_grammar::glob::Glob;

/// A Fluent file pattern containing `{locale}`, such as `"./l10n/{locale}.ftl"` or
/// `"./**/l10n/{locale}/**/*.ftl"`.
///
/// Exactly one `{locale}` placeholder is required. It matches a locale name within a
/// single path component. The other pattern syntax is described on [`Glob`].
#[derive(Clone)]
pub struct PathPattern {
    /// The original path string.
    pub lit: LitStr,
    /// The parsed pattern.
    pub glob: Glob,
}

impl Parse for PathPattern {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lit: LitStr = input.parse()?;
        let glob = Glob::parse(&lit.value(), Some("locale"))
            .map_err(|message| syn::Error::new(lit.span(), message))?;
        Ok(Self { lit, glob })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(path: &str) -> syn::Result<PathPattern> {
        syn::parse_str(&format!("{path:?}"))
    }

    #[test]
    fn requires_exactly_one_locale_placeholder() {
        assert!(parse("./l10n/{locale}.ftl").is_ok());
        assert!(parse("./l10n/en.ftl").is_err());
        assert!(parse("./l10n/{locale}/{locale}.ftl").is_err());
        assert!(parse("./l10n/{lang}.ftl").is_err());
    }
}
