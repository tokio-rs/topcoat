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

/// A Unicode language identifier, such as `en`, `de-AT`, or `zh-Hant-TW`.
///
/// It names a [`Language`] with optional [`Script`], [`Region`], and
/// [`Variant`] subtags, and is what translations are keyed by. Create one with
/// the [`langid!`](crate::langid) macro for text known at compile time, or
/// parse one with [`str::parse`]. Parsing canonicalizes letter case and
/// variant order, so `de-at` and `de-AT` are the same identifier. It keeps
/// deprecated subtags such as `iw` for Hebrew; replace those with
/// [`canonicalize`](Self::canonicalize).
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct LanguageIdentifier(icu_locale_core::LanguageIdentifier);

impl LanguageIdentifier {
    /// The identifier of the unknown language, written `und`.
    pub const UNKNOWN: Self = Self(icu_locale_core::LanguageIdentifier::UNKNOWN);

    /// Creates an identifier for `language` with no other subtags.
    #[must_use]
    pub const fn new(language: Language) -> Self {
        Self(icu_locale_core::LanguageIdentifier {
            language: language.to_icu(),
            script: None,
            region: None,
            variants: Variants::new(),
        })
    }

    /// Parses an identifier of the form `language[-script][-region][-variant]*`,
    /// canonicalizing letter case and variant order.
    ///
    /// # Errors
    ///
    /// Fails when the language is missing or malformed, or a subtag fits no
    /// position.
    pub fn try_from_str(text: &str) -> Result<Self, LocaleParseError> {
        icu_locale_core::LanguageIdentifier::try_from_str(text)
            .map(Self)
            .map_err(LocaleParseError::from_icu)
    }

    /// Parses like [`try_from_str`](Self::try_from_str) in const context,
    /// accepting at most one variant. Panics with `message` on failure.
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

    /// Returns the language subtag.
    #[must_use]
    pub const fn language(&self) -> Language {
        Language::from_icu(self.0.language)
    }

    /// Returns the script subtag, if present.
    #[must_use]
    pub const fn script(&self) -> Option<Script> {
        match self.0.script {
            Some(script) => Some(Script::from_icu(script)),
            None => None,
        }
    }

    /// Returns the region subtag, if present.
    #[must_use]
    pub const fn region(&self) -> Option<Region> {
        match self.0.region {
            Some(region) => Some(Region::from_icu(region)),
            None => None,
        }
    }

    /// Returns the variant subtags in canonical order.
    pub fn variants(&self) -> impl Iterator<Item = Variant> + '_ {
        self.0.variants.iter().copied().map(Variant::from_icu)
    }

    /// Returns whether the identifier has any variant subtags.
    #[must_use]
    pub const fn has_variants(&self) -> bool {
        !self.0.variants.is_empty()
    }

    /// Returns this identifier with `script` in place of its script subtag.
    #[must_use]
    pub fn with_script(mut self, script: Option<Script>) -> Self {
        self.0.script = script.map(Script::to_icu);
        self
    }

    /// Returns this identifier with `region` in place of its region subtag.
    #[must_use]
    pub fn with_region(mut self, region: Option<Region>) -> Self {
        self.0.region = region.map(Region::to_icu);
        self
    }

    /// Returns this identifier with `variant` added in canonical position.
    /// Adding a variant that is already present has no effect.
    #[must_use]
    pub fn with_variant(mut self, variant: Variant) -> Self {
        self.0.variants.push(variant.to_icu());
        self
    }

    /// Returns this identifier without any variant subtags.
    #[must_use]
    pub fn without_variants(mut self) -> Self {
        self.0.variants.clear();
        self
    }

    /// Replaces deprecated and aliased subtags with their current
    /// equivalents, so `iw` becomes `he` and `sh` becomes `sr-Latn`.
    ///
    /// Identifiers that are already canonical are returned unchanged.
    #[must_use]
    pub fn canonicalize(self) -> Self {
        let mut locale = icu_locale_core::Locale::from(self.0);
        LocaleCanonicalizer::new_common().canonicalize(&mut locale);
        Self(locale.id)
    }

    /// Fills in the most likely script and region for the language, so `en`
    /// becomes `en-Latn-US` and `zh-TW` becomes `zh-Hant-TW`.
    ///
    /// Subtags that are already present are kept.
    #[must_use]
    pub fn maximize(mut self) -> Self {
        LocaleExpander::new_common().maximize(&mut self.0);
        self
    }

    /// Returns the direction of the identifier's script, filling in the most
    /// likely script when none is given. Scripts without a direction are
    /// treated as left to right.
    #[must_use]
    pub fn direction(&self) -> Direction {
        if LocaleDirectionality::new_common().is_right_to_left(&self.0) {
            Direction::Rtl
        } else {
            Direction::Ltr
        }
    }

    /// Borrows the `icu_locale_core` identifier.
    #[must_use]
    pub const fn as_icu(&self) -> &icu_locale_core::LanguageIdentifier {
        &self.0
    }

    /// Converts to the `icu_locale_core` identifier.
    #[must_use]
    pub fn into_icu(self) -> icu_locale_core::LanguageIdentifier {
        self.0
    }

    /// Converts from the `icu_locale_core` identifier.
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

/// Creates a [`LanguageIdentifier`] from text known at compile time.
///
/// Malformed text is a compile error, and so is more than one variant
/// subtag. Parse identifiers with several variants at runtime instead. The
/// macro is usable in `const` and `static` items.
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
        for id in [langid!("ar"), langid!("he"), langid!("fa-IR"), langid!("ur"), langid!("az-Arab")] {
            assert_eq!(id.direction(), Direction::Rtl, "{id}");
        }
        for id in [langid!("en"), langid!("ja"), langid!("az"), langid!("az-Latn"), LanguageIdentifier::UNKNOWN] {
            assert_eq!(id.direction(), Direction::Ltr, "{id}");
        }
    }

    #[test]
    fn unknown_identifier_is_the_unknown_language() {
        assert_eq!(LanguageIdentifier::UNKNOWN, parse("und"));
        assert!(LanguageIdentifier::UNKNOWN.language().is_unknown());
    }
}
