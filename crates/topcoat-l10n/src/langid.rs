mod language;
mod region;
mod script;
mod variant;

use std::{
    fmt::{self, Debug, Display},
    str::FromStr,
};

use icu_locale::{LocaleCanonicalizer, LocaleDirectionality, LocaleExpander};
use icu_locale_core::subtags::Variants;
pub use language::*;
pub use region::*;
pub use script::*;
pub use variant::*;

use crate::{Direction, LocaleParseError};

/// A language code with optional script, region and variant details, such as `en`,
/// `de-AT` or `zh-Hant-TW`.
///
/// Translations are identified by this combination of [`Language`], [`Script`],
/// [`Region`] and [`Variant`] subtags. Use [`langid!`](crate::langid) for fixed text or
/// [`str::parse`] for runtime input.
///
/// Parsing normalizes letter case and variant order, so `de-at` and `de-AT` are equal.
/// It keeps older codes such as `iw` for Hebrew. Call
/// [`canonicalize`](Self::canonicalize) to replace them with current codes.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct LanguageIdentifier(icu_locale_core::LanguageIdentifier);

impl LanguageIdentifier {
    /// The `und` identifier for an unknown language.
    pub const UNKNOWN: Self = Self(icu_locale_core::LanguageIdentifier::UNKNOWN);

    /// Creates an identifier containing only the given language.
    #[must_use]
    pub const fn new(language: Language) -> Self {
        Self(icu_locale_core::LanguageIdentifier {
            language: language.to_icu(),
            script: None,
            region: None,
            variants: Variants::new(),
        })
    }

    /// Parses `language[-script][-region][-variant]*` and normalizes case and variant
    /// order.
    ///
    /// # Errors
    ///
    /// Returns an error if the language is missing or invalid, or a subtag cannot be
    /// used in the identifier.
    pub fn try_from_str(text: &str) -> Result<Self, LocaleParseError> {
        icu_locale_core::LanguageIdentifier::try_from_str(text)
            .map(Self)
            .map_err(LocaleParseError::from_icu)
    }

    /// Parses an identifier at compile time using the rules of
    /// [`try_from_str`](Self::try_from_str), with at most one variant. Panics with
    /// `message` if parsing fails.
    #[doc(hidden)]
    #[must_use]
    pub const fn parse_const(text: &str, message: &str) -> Self {
        match icu_locale_core::LanguageIdentifier::try_from_utf8_with_single_variant(
            text.as_bytes(),
        ) {
            Ok((language, script, region, variant)) => Self(icu_locale_core::LanguageIdentifier {
                language,
                script,
                region,
                variants: match variant {
                    Some(variant) => Variants::from_variant(variant),
                    None => Variants::new(),
                },
            }),
            Err(_) => panic!("{}", message),
        }
    }

    /// Returns the identifier's language code.
    #[must_use]
    pub const fn language(&self) -> Language {
        Language::from_icu(self.0.language)
    }

    /// Returns the script code, when specified.
    #[must_use]
    pub const fn script(&self) -> Option<Script> {
        match self.0.script {
            Some(script) => Some(Script::from_icu(script)),
            None => None,
        }
    }

    /// Returns the region code, when specified.
    #[must_use]
    pub const fn region(&self) -> Option<Region> {
        match self.0.region {
            Some(region) => Some(Region::from_icu(region)),
            None => None,
        }
    }

    /// Lists variants in their normalized order.
    pub fn variants(&self) -> impl Iterator<Item = Variant> + '_ {
        self.0.variants.iter().copied().map(Variant::from_icu)
    }

    /// Checks whether any variants are present.
    #[must_use]
    pub const fn has_variants(&self) -> bool {
        !self.0.variants.is_empty()
    }

    /// Replaces the script and returns the updated identifier.
    #[must_use]
    pub fn with_script(mut self, script: Option<Script>) -> Self {
        self.0.script = script.map(Script::to_icu);
        self
    }

    /// Replaces the region and returns the updated identifier.
    #[must_use]
    pub fn with_region(mut self, region: Option<Region>) -> Self {
        self.0.region = region.map(Region::to_icu);
        self
    }

    /// Adds a variant in normalized order and returns the updated identifier. Existing
    /// variants are not duplicated.
    #[must_use]
    pub fn with_variant(mut self, variant: Variant) -> Self {
        self.0.variants.push(variant.to_icu());
        self
    }

    /// Removes all variants and returns the updated identifier.
    #[must_use]
    pub fn without_variants(mut self) -> Self {
        self.0.variants.clear();
        self
    }

    /// Updates old or alternative codes to their standard forms. For example, `iw`
    /// becomes `he` and `sh` becomes `sr-Latn`.
    ///
    /// Leaves identifiers unchanged when they are already canonical.
    #[must_use]
    pub fn canonicalize(self) -> Self {
        let mut locale = icu_locale_core::Locale::from(self.0);
        LocaleCanonicalizer::new_common().canonicalize(&mut locale);
        Self(locale.id)
    }

    /// Adds the most likely missing script and region. For example, `en` becomes
    /// `en-Latn-US` and `zh-TW` becomes `zh-Hant-TW`.
    ///
    /// Keeps subtags that are already present.
    #[must_use]
    pub fn maximize(mut self) -> Self {
        LocaleExpander::new_common().maximize(&mut self.0);
        self
    }

    /// Returns the script's writing direction. If no script is specified, uses the most
    /// likely one for the language. Defaults to left to right for scripts without a
    /// defined direction.
    #[must_use]
    pub fn direction(&self) -> Direction {
        if LocaleDirectionality::new_common().is_right_to_left(&self.0) {
            Direction::Rtl
        } else {
            Direction::Ltr
        }
    }

    /// Returns a reference to the underlying `icu_locale_core` identifier.
    #[must_use]
    pub const fn as_icu(&self) -> &icu_locale_core::LanguageIdentifier {
        &self.0
    }

    /// Returns the identifier as an `icu_locale_core` value.
    #[must_use]
    pub fn into_icu(self) -> icu_locale_core::LanguageIdentifier {
        self.0
    }

    /// Wraps a language identifier from `icu_locale_core`.
    #[must_use]
    pub const fn from_icu(identifier: icu_locale_core::LanguageIdentifier) -> Self {
        Self(identifier)
    }
}

impl Debug for LanguageIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("LanguageIdentifier")
            .field(&self.to_string())
            .finish()
    }
}

impl Display for LanguageIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl FromStr for LanguageIdentifier {
    type Err = LocaleParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::try_from_str(text)
    }
}

impl From<Language> for LanguageIdentifier {
    fn from(language: Language) -> Self {
        Self::new(language)
    }
}

/// Checks a fixed language identifier at compile time and creates a
/// [`LanguageIdentifier`].
///
/// Supports at most one variant subtag. Invalid text or additional variants cause a
/// compile error. Parse identifiers with several variants at runtime instead. Works in
/// `const` and `static` items.
///
/// # Examples
///
/// ```rust
/// use topcoat::l10n::{LanguageIdentifier, langid, region};
///
/// const DEFAULT: LanguageIdentifier = langid!("en-US");
///
/// assert_eq!(DEFAULT.region(), Some(region!("US")));
/// ```
#[macro_export]
macro_rules! langid {
    ($text:literal) => {
        const {
            $crate::LanguageIdentifier::parse_const(
                $text,
                concat!(
                    "invalid language identifier or more than one variant: ",
                    $text
                ),
            )
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{language, region, script, variant};

    fn parse(text: &str) -> LanguageIdentifier {
        text.parse().unwrap()
    }

    #[test]
    fn parses_each_position() {
        let id = parse("zh-Hant-TW-posix");

        assert_eq!(id.language(), language!("zh"));
        assert_eq!(id.script(), Some(script!("Hant")));
        assert_eq!(id.region(), Some(region!("TW")));
        assert_eq!(id.variants().collect::<Vec<_>>(), [variant!("posix")]);
    }

    #[test]
    fn positions_are_optional() {
        assert_eq!(parse("en").script(), None);
        assert_eq!(parse("en").region(), None);
        assert!(!parse("en").has_variants());
        assert_eq!(parse("en-US").script(), None);
        assert_eq!(parse("en-Latn").region(), None);
        assert_eq!(parse("de-1996").region(), None);
        assert_eq!(parse("es-419").region(), Some(region!("419")));
    }

    #[test]
    fn canonicalizes_case_and_variant_order() {
        assert_eq!(parse("ZH-hant-tw"), langid!("zh-Hant-TW"));
        assert_eq!(parse("sl-rozaj-1994").to_string(), "sl-1994-rozaj");
        assert_eq!(parse("sl-1994-rozaj"), parse("sl-rozaj-1994"));
    }

    #[test]
    fn display_round_trips() {
        for text in ["en", "en-US", "zh-Hant-TW", "de-CH-1901", "sl-1994-rozaj"] {
            assert_eq!(parse(text).to_string(), text);
        }
    }

    #[test]
    fn rejects_malformed_text() {
        for text in ["", "-US", "en-", "en--US", "en-US-Latn", "en-US-DE", "en-Ü"] {
            assert!(text.parse::<LanguageIdentifier>().is_err(), "{text}");
        }
        assert_eq!(
            "1en".parse::<LanguageIdentifier>(),
            Err(LocaleParseError::InvalidLanguage)
        );
    }

    #[test]
    fn macro_matches_runtime_parsing() {
        const PARSED: LanguageIdentifier = langid!("zh-hant-tw-POSIX");

        assert_eq!(PARSED, parse("zh-Hant-TW-posix"));
    }

    #[test]
    fn builders_replace_subtags() {
        let id = parse("de-Latn-AT-1996");

        assert_eq!(id.clone().with_region(None).to_string(), "de-Latn-1996");
        assert_eq!(id.clone().with_script(None).to_string(), "de-AT-1996");
        assert_eq!(id.clone().without_variants().to_string(), "de-Latn-AT");
        assert_eq!(
            id.clone().with_variant(variant!("1901")).to_string(),
            "de-Latn-AT-1901-1996"
        );
        assert_eq!(id.clone().with_variant(variant!("1996")), id);
    }

    #[test]
    fn canonicalize_replaces_deprecated_subtags() {
        assert_eq!(langid!("iw").canonicalize(), langid!("he"));
        assert_eq!(langid!("en-UK").canonicalize(), langid!("en-GB"));
        assert_eq!(langid!("sh-RS").canonicalize(), langid!("sr-Latn-RS"));
    }

    #[test]
    fn canonicalize_keeps_canonical_identifiers() {
        for id in [
            langid!("en-US"),
            langid!("zh-Hant-TW"),
            langid!("de-CH-1901"),
            LanguageIdentifier::UNKNOWN,
        ] {
            assert_eq!(id.clone().canonicalize(), id, "{id}");
        }
    }

    #[test]
    fn maximize_fills_in_likely_subtags() {
        assert_eq!(langid!("en").maximize(), langid!("en-Latn-US"));
        assert_eq!(langid!("zh-TW").maximize(), langid!("zh-Hant-TW"));
        assert_eq!(langid!("sr-Latn").maximize(), langid!("sr-Latn-RS"));
        assert_eq!(langid!("en-GB").maximize(), langid!("en-Latn-GB"));
    }

    #[test]
    fn direction_follows_the_script() {
        for id in [
            langid!("ar"),
            langid!("he"),
            langid!("fa-IR"),
            langid!("ur"),
            langid!("az-Arab"),
        ] {
            assert_eq!(id.direction(), Direction::Rtl, "{id}");
        }
        for id in [
            langid!("en"),
            langid!("ja"),
            langid!("az"),
            langid!("az-Latn"),
            LanguageIdentifier::UNKNOWN,
        ] {
            assert_eq!(id.direction(), Direction::Ltr, "{id}");
        }
    }

    #[test]
    fn unknown_identifier_is_the_unknown_language() {
        assert_eq!(LanguageIdentifier::UNKNOWN, parse("und"));
        assert!(LanguageIdentifier::UNKNOWN.language().is_unknown());
    }
}
