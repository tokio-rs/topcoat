use std::{
    fmt::{self, Debug, Display},
    str::FromStr,
};

use icu_locale::LocaleCanonicalizer;
use icu_locale_core::extensions::unicode::{Attributes, Keywords, Unicode, Value};

use crate::{Extensions, LanguageIdentifier, LocaleParseError};

/// A Unicode locale identifier, such as `en-US` or `th-TH-u-ca-buddhist`.
///
/// It is a [`LanguageIdentifier`] followed by optional [`Extensions`]. Create
/// one with the [`locale!`](crate::locale) macro for text known at compile
/// time, or parse one with [`str::parse`]. Parsing canonicalizes letter case
/// and the order of variants and extensions, but keeps deprecated subtags;
/// replace those with [`canonicalize`](Self::canonicalize).
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Locale {
    id: LanguageIdentifier,
    extensions: Extensions,
}

impl Locale {
    /// The locale of the unknown language, written `und`.
    pub const UNKNOWN: Self = Self::new(LanguageIdentifier::UNKNOWN);

    /// Creates a locale for `id` with no extensions.
    #[must_use]
    pub const fn new(id: LanguageIdentifier) -> Self {
        Self {
            id,
            extensions: Extensions::new(),
        }
    }

    /// Parses a locale of the form `language[-script][-region][-variant]*`
    /// followed by any extensions, canonicalizing letter case and order.
    ///
    /// # Errors
    ///
    /// Fails when the language identifier or an extension is malformed, or an
    /// extension appears twice.
    pub fn try_from_str(text: &str) -> Result<Self, LocaleParseError> {
        icu_locale_core::Locale::try_from_str(text)
            .map(Self::from_icu)
            .map_err(LocaleParseError::from_icu)
    }

    /// Parses like [`try_from_str`](Self::try_from_str) in const context,
    /// accepting at most one variant and one Unicode extension keyword.
    /// Panics with `message` on failure.
    #[doc(hidden)]
    #[must_use]
    pub const fn parse_const(text: &str, message: &str) -> Self {
        let Ok((language, script, region, variant, keyword)) =
            icu_locale_core::Locale::try_from_utf8_with_single_variant_single_keyword_unicode_extension(
                text.as_bytes(),
            )
        else {
            panic!("{}", message);
        };
        let id = LanguageIdentifier::from_icu(icu_locale_core::LanguageIdentifier {
            language,
            script,
            region,
            variants: match variant {
                Some(variant) => icu_locale_core::subtags::Variants::from_variant(variant),
                None => icu_locale_core::subtags::Variants::new(),
            },
        });
        let extensions = match keyword {
            Some((key, value)) => icu_locale_core::extensions::Extensions::from_unicode(Unicode {
                keywords: Keywords::new_single(key, Value::from_subtag(value)),
                attributes: Attributes::new(),
            }),
            None => icu_locale_core::extensions::Extensions::new(),
        };
        Self {
            id,
            extensions: Extensions::from_icu(extensions),
        }
    }

    /// Returns the language identifier.
    #[must_use]
    pub const fn id(&self) -> &LanguageIdentifier {
        &self.id
    }

    /// Returns the extensions.
    #[must_use]
    pub const fn extensions(&self) -> &Extensions {
        &self.extensions
    }

    /// Returns this locale with `id` in place of its language identifier.
    #[must_use]
    pub fn with_id(mut self, id: LanguageIdentifier) -> Self {
        self.id = id;
        self
    }

    /// Returns this locale with `extensions` in place of its extensions.
    #[must_use]
    pub fn with_extensions(mut self, extensions: Extensions) -> Self {
        self.extensions = extensions;
        self
    }

    /// Replaces deprecated and aliased subtags with their current
    /// equivalents, so `iw` becomes `he` and `sh` becomes `sr-Latn`.
    ///
    /// Locales that are already canonical are returned unchanged.
    #[must_use]
    pub fn canonicalize(self) -> Self {
        let mut locale = self.into_icu();
        LocaleCanonicalizer::new_common().canonicalize(&mut locale);
        Self::from_icu(locale)
    }

    /// Converts to the `icu_locale_core` locale.
    #[must_use]
    pub fn into_icu(self) -> icu_locale_core::Locale {
        icu_locale_core::Locale {
            id: self.id.into_icu(),
            extensions: self.extensions.into_icu(),
        }
    }

    /// Converts from the `icu_locale_core` locale.
    #[must_use]
    pub fn from_icu(locale: icu_locale_core::Locale) -> Self {
        Self {
            id: LanguageIdentifier::from_icu(locale.id),
            extensions: Extensions::from_icu(locale.extensions),
        }
    }
}

impl Debug for Locale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Locale").field(&self.to_string()).finish()
    }
}

impl Display for Locale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.id, f)?;
        if !self.extensions.is_empty() {
            write!(f, "-{}", self.extensions)?;
        }
        Ok(())
    }
}

impl FromStr for Locale {
    type Err = LocaleParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::try_from_str(text)
    }
}

impl From<LanguageIdentifier> for Locale {
    fn from(id: LanguageIdentifier) -> Self {
        Self::new(id)
    }
}

/// Creates a [`Locale`] from text known at compile time.
///
/// Malformed text is a compile error, and so are more than one variant
/// subtag and extensions other than a single Unicode extension keyword. Parse
/// other locales at runtime instead. The macro is usable in `const` and
/// `static` items.
///
/// # Examples
///
/// ```rust
/// use topcoat::l10n::{Locale, langid, locale};
///
/// const THAI: Locale = locale!("th-TH-u-ca-buddhist");
///
/// assert_eq!(THAI.id(), &langid!("th-TH"));
/// ```
#[macro_export]
macro_rules! locale {
    ($text:literal) => {
        const {
            $crate::Locale::parse_const(
                $text,
                concat!(
                    "invalid locale or more than one variant or keyword: ",
                    $text
                ),
            )
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::langid;

    fn parse(text: &str) -> Locale {
        text.parse().unwrap()
    }

    #[test]
    fn separates_the_identifier_from_the_extensions() {
        let locale = parse("th-TH-u-ca-buddhist");

        assert_eq!(locale.id(), &langid!("th-TH"));
        assert_eq!(locale.extensions().to_string(), "u-ca-buddhist");
        assert!(parse("th-TH").extensions().is_empty());
    }

    #[test]
    fn canonicalizes_case_and_keyword_order() {
        assert_eq!(
            parse("EN-us-U-HC-H23-CA-Buddhist").to_string(),
            "en-US-u-ca-buddhist-hc-h23"
        );
    }

    #[test]
    fn display_round_trips() {
        for text in [
            "en",
            "en-US-u-ca-buddhist",
            "de-CH-1901-u-co-phonebk",
            "en-t-de-h0-hybrid",
            "en-a-bar-u-ca-gregory-x-foo",
        ] {
            assert_eq!(parse(text).to_string(), text);
        }
    }

    #[test]
    fn rejects_malformed_text() {
        for text in ["", "en-u", "en-u-ca-buddhist-u-hc-h23", "en-x"] {
            assert!(text.parse::<Locale>().is_err(), "{text}");
        }
    }

    #[test]
    fn macro_matches_runtime_parsing() {
        const PARSED: Locale = locale!("de-at-posix-U-CA-Buddhist");

        assert_eq!(PARSED, parse("de-AT-posix-u-ca-buddhist"));
        assert_eq!(locale!("de"), Locale::from(langid!("de")));
    }

    #[test]
    fn canonicalize_replaces_deprecated_subtags_and_keeps_extensions() {
        assert_eq!(
            locale!("iw-u-ca-hebrew").canonicalize(),
            locale!("he-u-ca-hebrew")
        );
    }
}
