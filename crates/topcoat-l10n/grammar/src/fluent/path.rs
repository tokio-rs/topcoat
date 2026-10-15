use proc_macro2::Span;
use syn::{
    LitStr,
    parse::{Parse, ParseStream},
};
use topcoat_core_grammar::ParseOption;

const PLACEHOLDER: &str = "{locale}";
const MODULE: &str = "{mod}";
const DEFAULT: &str = "{mod}/l10n/{locale}.ftl";

/// A path to Fluent files with a `{locale}` placeholder, such as
/// `"./l10n/{locale}.ftl"` or `"./**/l10n/{locale}/**/*.ftl"`.
///
/// The path contains exactly one `{locale}` placeholder, which matches the
/// locale within a single path component. The path may also contain these
/// wildcards:
///
/// - `*` matches any characters within a single path component.
/// - `**` matches zero or more directories and must be a whole path component.
///
/// The path may start with a `{mod}` component, which stands for the directory
/// of the invoking module. That is the directory where its child modules live,
/// such as `src/about/` for both `src/about.rs` and `src/about/mod.rs`.
///
/// `*` cannot appear in the same path component as `{locale}`. Other braces,
/// `?`, `[` and `]` are reserved and rejected.
#[derive(Clone)]
pub struct PathPattern {
    /// The path as written.
    pub lit: LitStr,
}

impl PathPattern {
    /// Returns the `/`-separated components of the path.
    ///
    /// # Panics
    ///
    /// Panics if the path is not a valid pattern. A parsed `PathPattern` is
    /// always valid.
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

/// The default path, `"{mod}/l10n/{locale}.ftl"`.
impl Default for PathPattern {
    fn default() -> Self {
        Self {
            lit: LitStr::new(DEFAULT, Span::call_site()),
        }
    }
}

impl ParseOption for PathPattern {
    fn peek(input: ParseStream) -> bool {
        input.peek(LitStr)
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

/// One `/`-separated component of a [`PathPattern`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathComponent {
    /// `{mod}`, the directory of the invoking module.
    Module,
    /// A name without wildcards or placeholders.
    Literal(String),
    /// A name containing `*` wildcards, as written.
    Wildcard(String),
    /// `**`, matching zero or more directories.
    Recursive,
    /// A name containing the `{locale}` placeholder, split into the text
    /// before and after it.
    Locale {
        /// The text before the placeholder.
        prefix: String,
        /// The text after the placeholder.
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
    fn default_is_a_valid_pattern() {
        // `components` panics on an invalid pattern.
        assert_eq!(
            PathPattern::default().components().first(),
            Some(&PathComponent::Module)
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
