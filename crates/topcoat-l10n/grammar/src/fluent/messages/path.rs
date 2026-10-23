mod resolve;

pub use resolve::*;
use syn::{
    LitStr,
    parse::{Parse, ParseStream},
};

const PLACEHOLDER: &str = "{locale}";
const MODULE: &str = "{mod}";

/// A Fluent file pattern containing `{locale}`, such as `"./l10n/{locale}.ftl"` or
/// `"./**/l10n/{locale}/**/*.ftl"`.
///
/// Exactly one `{locale}` placeholder is required. It matches a locale name within a
/// single path component.
///
/// - `*` matches any characters within one path component.
/// - `**` matches any number of directories, including none, and must occupy a whole
/// component.
///
/// A leading `{mod}` uses the directory where the current module's child modules
/// belong. For both `src/about.rs` and `src/about/mod.rs`, this is `src/about/`.
///
/// A component containing `{locale}` cannot also contain `*`. Other braces and the
/// characters `?`, `[` and `]` are not supported.
#[derive(Clone)]
pub struct PathPattern {
    /// The original path string.
    pub lit: LitStr,
}

impl PathPattern {
    /// Splits the pattern into components at `/` separators.
    ///
    /// # Panics
    ///
    /// Panics for an invalid pattern. Successfully parsed patterns are always valid.
    #[must_use]
    pub fn components(&self) -> Vec<PathComponent> {
        Self::parse_components(&self.lit.value()).expect("the path is validated during parsing")
    }

    fn parse_components(path: &str) -> Result<Vec<PathComponent>, &'static str> {
        match path.matches(PLACEHOLDER).count() {
            0 => return Err("expected a `{locale}` placeholder in the path"),
            1 => {}
            _ => return Err("expected exactly one `{locale}` placeholder in the path"),
        }
        path.split('/')
            .enumerate()
            .map(|(index, component)| match component {
                MODULE if index == 0 => Ok(PathComponent::Module),
                _ if component.contains(MODULE) => {
                    Err("`{mod}` must be the whole first path component")
                }
                _ => PathComponent::parse(component),
            })
            .collect()
    }
}

impl Parse for PathPattern {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lit: LitStr = input.parse()?;
        Self::parse_components(&lit.value())
            .map_err(|message| syn::Error::new(lit.span(), message))?;
        Ok(Self { lit })
    }
}

/// One part of a [`PathPattern`], separated from others by `/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathComponent {
    /// The `{mod}` placeholder for the current module's directory.
    Module,
    /// A literal file or directory name.
    Literal(String),
    /// A file or directory pattern containing `*`.
    Wildcard(String),
    /// The `**` wildcard for any number of directories.
    Recursive,
    /// A component containing `{locale}`, stored as the text on either side.
    Locale {
        /// Text immediately before `{locale}`.
        prefix: String,
        /// Text immediately after `{locale}`.
        suffix: String,
    },
}

impl PathComponent {
    fn parse(component: &str) -> Result<Self, &'static str> {
        if component.contains(['?', '[', ']']) {
            return Err("`?`, `[` and `]` are not supported in the path");
        }
        if component == "**" {
            return Ok(Self::Recursive);
        }
        if component.contains("**") {
            return Err("`**` must be a whole path component");
        }
        if let Some((prefix, suffix)) = component.split_once(PLACEHOLDER) {
            if prefix.contains('*') || suffix.contains('*') {
                return Err("`*` cannot appear in the same path component as `{locale}`");
            }
            if prefix.contains(['{', '}']) || suffix.contains(['{', '}']) {
                return Err("unexpected brace in the path; only `{locale}` is supported");
            }
            return Ok(Self::Locale {
                prefix: prefix.to_owned(),
                suffix: suffix.to_owned(),
            });
        }
        if component.contains(['{', '}']) {
            return Err("unexpected brace in the path; only `{locale}` is supported");
        }
        if component.contains('*') {
            Ok(Self::Wildcard(component.to_owned()))
        } else {
            Ok(Self::Literal(component.to_owned()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(path: &str) -> syn::Result<PathPattern> {
        syn::parse_str(&format!("{path:?}"))
    }

    fn literal(name: &str) -> PathComponent {
        PathComponent::Literal(name.to_owned())
    }

    #[test]
    fn splits_a_placeholder_out_of_a_file_name() {
        let pattern = parse("./l10n/messages-{locale}.ftl").unwrap();
        assert_eq!(
            pattern.components(),
            [
                literal("."),
                literal("l10n"),
                PathComponent::Locale {
                    prefix: "messages-".to_owned(),
                    suffix: ".ftl".to_owned(),
                },
            ]
        );
    }

    #[test]
    fn accepts_wildcards_around_the_placeholder() {
        let pattern = parse("./**/l10n/{locale}/**/*.ftl").unwrap();
        assert_eq!(
            pattern.components(),
            [
                literal("."),
                PathComponent::Recursive,
                literal("l10n"),
                PathComponent::Locale {
                    prefix: String::new(),
                    suffix: String::new(),
                },
                PathComponent::Recursive,
                PathComponent::Wildcard("*.ftl".to_owned()),
            ]
        );
    }

    #[test]
    fn accepts_a_leading_module_directory() {
        let pattern = parse("{mod}/l10n/{locale}.ftl").unwrap();
        assert_eq!(
            pattern.components(),
            [
                PathComponent::Module,
                literal("l10n"),
                PathComponent::Locale {
                    prefix: String::new(),
                    suffix: ".ftl".to_owned(),
                },
            ]
        );
    }

    #[test]
    fn rejects_a_module_directory_after_the_start() {
        assert!(parse("./{mod}/{locale}.ftl").is_err());
        assert!(parse("l10n/{mod}/{locale}.ftl").is_err());
    }

    #[test]
    fn rejects_a_module_directory_within_a_component() {
        assert!(parse("{mod}-x/{locale}.ftl").is_err());
        assert!(parse("{mod}{locale}.ftl").is_err());
    }

    #[test]
    fn rejects_a_missing_placeholder() {
        assert!(parse("./l10n/en.ftl").is_err());
    }

    #[test]
    fn rejects_multiple_placeholders() {
        assert!(parse("./l10n/{locale}/{locale}.ftl").is_err());
        assert!(parse("./l10n/{locale}{locale}.ftl").is_err());
    }

    #[test]
    fn rejects_other_braces() {
        assert!(parse("./l10n/{locale}/{a,b}.ftl").is_err());
        assert!(parse("./l10n/{lang}/{locale}.ftl").is_err());
        assert!(parse("./l10n/{locale}}.ftl").is_err());
    }

    #[test]
    fn rejects_reserved_glob_syntax() {
        assert!(parse("./l10n/{locale}/v?.ftl").is_err());
        assert!(parse("./l10n/{locale}/[ab].ftl").is_err());
    }

    #[test]
    fn rejects_a_partial_recursive_wildcard() {
        assert!(parse("./l10n/{locale}/**.ftl").is_err());
        assert!(parse("./a**/{locale}.ftl").is_err());
    }

    #[test]
    fn rejects_a_wildcard_next_to_the_placeholder() {
        assert!(parse("./l10n/{locale}-*.ftl").is_err());
        assert!(parse("./l10n/*{locale}/main.ftl").is_err());
    }
}
