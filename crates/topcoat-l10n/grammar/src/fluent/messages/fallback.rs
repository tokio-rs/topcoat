use icu_locale_core::LanguageIdentifier;
use syn::{
    LitStr, Token,
    parse::{Parse, ParseStream},
};
use topcoat_core_grammar::ParseOption;

mod kw {
    use syn::custom_keyword;

    custom_keyword!(fallback);
}

/// The fallback locale of a `messages!` invocation, written
/// `fallback = "en"`.
///
/// Messages that a locale lacks are taken from the fallback locale.
pub struct MessagesFallback {
    /// The `fallback` keyword.
    pub fallback_token: kw::fallback,
    /// The `=` after the keyword.
    pub eq_token: Token![=],
    /// The locale as written.
    pub locale: LitStr,
}

impl MessagesFallback {
    /// Returns the fallback locale.
    ///
    /// # Panics
    ///
    /// Panics if the locale is invalid. A parsed `MessagesFallback` always
    /// has a valid locale.
    #[must_use]
    pub fn locale(&self) -> LanguageIdentifier {
        LanguageIdentifier::try_from_str(&self.locale.value())
            .expect("the locale is validated during parsing")
    }
}

impl ParseOption for MessagesFallback {
    fn peek(input: ParseStream) -> bool {
        input.peek(kw::fallback)
    }
}

impl Parse for MessagesFallback {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let fallback = Self {
            fallback_token: input.parse()?,
            eq_token: input.parse()?,
            locale: input.parse()?,
        };
        LanguageIdentifier::try_from_str(&fallback.locale.value()).map_err(|_| {
            syn::Error::new(
                fallback.locale.span(),
                "expected a locale, such as `en` or `de-AT`",
            )
        })?;
        Ok(fallback)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> syn::Result<MessagesFallback> {
        syn::parse_str(input)
    }

    #[test]
    fn parses_the_locale() {
        let fallback = parse(r#"fallback = "de-at""#).unwrap();
        assert_eq!(fallback.locale().to_string(), "de-AT");
        assert_eq!(fallback.locale.value(), "de-at");
    }

    #[test]
    fn rejects_an_invalid_locale() {
        assert!(parse(r#"fallback = "e""#).is_err());
    }

    #[test]
    fn rejects_a_missing_locale() {
        assert!(parse("fallback =").is_err());
        assert!(parse(r#"fallback "en""#).is_err());
    }
}
