use syn::{
    Visibility,
    parse::{Parse, ParseStream},
};
use topcoat_core_grammar::ParseOption;

use crate::fluent::PathPattern;

/// The input of a `messages!` invocation: an optional visibility followed by
/// an optional path of the Fluent files to load.
///
/// ```text
/// messages!(pub(crate) "./l10n/{locale}.ftl")
/// ```
pub struct Messages {
    /// The visibility of the generated module.
    pub vis: Visibility,
    /// The path of the Fluent files, if written.
    pub path: Option<PathPattern>,
}

impl Messages {
    /// Returns the path of the Fluent files, or the
    /// [default path](PathPattern::default) when none is written.
    #[must_use]
    pub fn path(&self) -> PathPattern {
        self.path.clone().unwrap_or_default()
    }
}

impl Parse for Messages {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            vis: input.parse()?,
            path: input.call(PathPattern::parse_option)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> syn::Result<Messages> {
        syn::parse_str(input)
    }

    #[test]
    fn uses_the_default_path_when_none_is_written() {
        let messages = parse("pub(crate)").unwrap();
        assert!(matches!(messages.vis, Visibility::Restricted(_)));
        assert_eq!(
            messages.path().lit.value(),
            PathPattern::default().lit.value()
        );
        assert!(parse("").is_ok());
    }

    #[test]
    fn uses_the_written_path() {
        let messages = parse(r#""./l10n/{locale}.ftl""#).unwrap();
        assert!(matches!(messages.vis, Visibility::Inherited));
        assert_eq!(messages.path().lit.value(), "./l10n/{locale}.ftl");
    }

    #[test]
    fn rejects_trailing_input() {
        assert!(parse(r#""./l10n/{locale}.ftl" "./other/{locale}.ftl""#).is_err());
    }
}
