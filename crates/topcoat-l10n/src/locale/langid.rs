mod common;
mod language;
mod region;
mod script;
mod variant;

use std::{
    fmt::{self, Debug, Display},
    str::FromStr,
};

pub use language::*;
pub use region::*;
pub use script::*;
pub use variant::*;

use crate::LocaleParseError;

/// The most variant subtags a [`LanguageIdentifier`] can hold.
pub const MAX_VARIANTS: usize = 4;

/// A Unicode language identifier, such as `en`, `de-AT`, or `zh-Hant-TW`.
///
/// It names a [`Language`] with optional [`Script`], [`Region`], and
/// [`Variant`] subtags, and is what translations are keyed by. Create one with
/// the [`langid!`](crate::langid) macro for text known at compile time, or
/// parse one with [`str::parse`]. Parsing canonicalizes letter case and
/// variant order, so `de-at` and `de-AT` are the same identifier.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct LanguageIdentifier {
    language: Language,
    script: Option<Script>,
    region: Option<Region>,
    variants: [Option<Variant>; MAX_VARIANTS],
}

impl LanguageIdentifier {
    /// The identifier of the unknown language, written `und`.
    pub const UNKNOWN: Self = Self::new(Language::UNKNOWN);

    /// Creates an identifier for `language` with no other subtags.
    #[must_use]
    pub const fn new(language: Language) -> Self {
        Self {
            language,
            script: None,
            region: None,
            variants: [None; MAX_VARIANTS],
        }
    }

    /// Parses an identifier of the form `language[-script][-region][-variant]*`,
    /// canonicalizing letter case and variant order.
    ///
    /// # Errors
    ///
    /// Fails when the language is missing or malformed, a subtag fits no
    /// position, a variant repeats, or there are more than [`MAX_VARIANTS`]
    /// variants.
    pub const fn try_from_str(text: &str) -> Result<Self, LocaleParseError> {
        let bytes = text.as_bytes();
        let mut start = 0;
        let mut index = 0;
        let mut identifier = None;
        let mut next = Position::Script;

        loop {
            if index == bytes.len() || bytes[index] == b'-' {
                let (_, rest) = bytes.split_at(start);
                let (piece, _) = rest.split_at(index - start);
                let Ok(piece) = str::from_utf8(piece) else {
                    return Err(LocaleParseError::InvalidSubtag);
                };

                identifier = match identifier {
                    None => match Language::try_from_str(piece) {
                        Ok(language) => Some(Self::new(language)),
                        Err(error) => return Err(error),
                    },
                    Some(identifier) => match identifier.push(piece, &mut next) {
                        Ok(identifier) => Some(identifier),
                        Err(error) => return Err(error),
                    },
                };

                if index == bytes.len() {
                    break;
                }
                start = index + 1;
            }
            index += 1;
        }

        match identifier {
            Some(identifier) => Ok(identifier),
            None => Err(LocaleParseError::InvalidLanguage),
        }
    }

    /// Places `piece` in the next open position after the language.
    const fn push(mut self, piece: &str, next: &mut Position) -> Result<Self, LocaleParseError> {
        if matches!(next, Position::Script)
            && let Ok(script) = Script::try_from_str(piece)
        {
            self.script = Some(script);
            *next = Position::Region;
            return Ok(self);
        }
        if matches!(next, Position::Script | Position::Region)
            && let Ok(region) = Region::try_from_str(piece)
        {
            self.region = Some(region);
            *next = Position::Variant;
            return Ok(self);
        }
        match Variant::try_from_str(piece) {
            Ok(variant) => {
                *next = Position::Variant;
                self.with_variant(variant)
            }
            Err(_) => Err(LocaleParseError::InvalidSubtag),
        }
    }

    /// Returns the language subtag.
    #[must_use]
    pub const fn language(&self) -> Language {
        self.language
    }

    /// Returns the script subtag, if present.
    #[must_use]
    pub const fn script(&self) -> Option<Script> {
        self.script
    }

    /// Returns the region subtag, if present.
    #[must_use]
    pub const fn region(&self) -> Option<Region> {
        self.region
    }

    /// Returns the variant subtags in canonical order.
    pub fn variants(&self) -> impl Iterator<Item = Variant> + '_ {
        self.variants.iter().flatten().copied()
    }

    /// Returns whether the identifier has any variant subtags.
    #[must_use]
    pub const fn has_variants(&self) -> bool {
        self.variants[0].is_some()
    }

    /// Returns this identifier with `script` in place of its script subtag.
    #[must_use]
    pub const fn with_script(mut self, script: Option<Script>) -> Self {
        self.script = script;
        self
    }

    /// Returns this identifier with `region` in place of its region subtag.
    #[must_use]
    pub const fn with_region(mut self, region: Option<Region>) -> Self {
        self.region = region;
        self
    }

    /// Returns this identifier with `variant` added in canonical position.
    ///
    /// # Errors
    ///
    /// Fails when the variant is already present or the identifier holds
    /// [`MAX_VARIANTS`] variants.
    pub const fn with_variant(mut self, variant: Variant) -> Result<Self, LocaleParseError> {
        let mut index = 0;
        while index < MAX_VARIANTS {
            match self.variants[index] {
                None => {
                    self.variants[index] = Some(variant);
                    return Ok(self);
                }
                Some(existing) => match variant.compare(&existing) {
                    std::cmp::Ordering::Equal => return Err(LocaleParseError::DuplicateVariant),
                    std::cmp::Ordering::Less => {
                        // Shift the tail right to keep the variants sorted.
                        if self.variants[MAX_VARIANTS - 1].is_some() {
                            return Err(LocaleParseError::TooManyVariants);
                        }
                        let mut tail = MAX_VARIANTS - 1;
                        while tail > index {
                            self.variants[tail] = self.variants[tail - 1];
                            tail -= 1;
                        }
                        self.variants[index] = Some(variant);
                        return Ok(self);
                    }
                    std::cmp::Ordering::Greater => {}
                },
            }
            index += 1;
        }
        Err(LocaleParseError::TooManyVariants)
    }

    /// Returns this identifier without any variant subtags.
    #[must_use]
    pub const fn without_variants(mut self) -> Self {
        self.variants = [None; MAX_VARIANTS];
        self
    }

    /// Converts to the `icu_locale_core` identifier.
    #[must_use]
    pub fn to_icu(&self) -> icu_locale_core::LanguageIdentifier {
        use icu_locale_core::subtags::Variants;

        let mut variants = self.variants().map(Variant::to_icu);
        let variants = match (variants.next(), variants.next()) {
            (None, _) => Variants::new(),
            (Some(first), None) => Variants::from_variant(first),
            (Some(first), Some(second)) => {
                // Ours are already sorted and unique, as the constructor requires.
                let mut all = vec![first, second];
                all.extend(variants);
                Variants::from_vec_unchecked(all)
            }
        };
        icu_locale_core::LanguageIdentifier {
            language: self.language.to_icu(),
            script: self.script.map(Script::to_icu),
            region: self.region.map(Region::to_icu),
            variants,
        }
    }

    /// Converts from the `icu_locale_core` identifier.
    ///
    /// # Errors
    ///
    /// Fails when the identifier has more than [`MAX_VARIANTS`] variants.
    pub fn try_from_icu(
        identifier: &icu_locale_core::LanguageIdentifier,
    ) -> Result<Self, LocaleParseError> {
        let mut converted = Self::new(Language::from_icu(identifier.language))
            .with_script(identifier.script.map(Script::from_icu))
            .with_region(identifier.region.map(Region::from_icu));
        for variant in identifier.variants.iter() {
            converted = converted.with_variant(Variant::from_icu(*variant))?;
        }
        Ok(converted)
    }
}

/// The next subtag position the parser may fill.
enum Position {
    Script,
    Region,
    Variant,
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
        f.write_str(self.language.as_str())?;
        if let Some(script) = self.script {
            write!(f, "-{script}")?;
        }
        if let Some(region) = self.region {
            write!(f, "-{region}")?;
        }
        for variant in self.variants.iter().flatten() {
            write!(f, "-{variant}")?;
        }
        Ok(())
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
/// Malformed text is a compile error. The macro is usable in `const` and
/// `static` items.
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
            match $crate::LanguageIdentifier::try_from_str($text) {
                Ok(identifier) => identifier,
                Err(_) => panic!(concat!("invalid language identifier: ", $text)),
            }
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
        assert_eq!(
            "".parse::<LanguageIdentifier>(),
            Err(LocaleParseError::InvalidLanguage)
        );
        assert_eq!(
            "-US".parse::<LanguageIdentifier>(),
            Err(LocaleParseError::InvalidLanguage)
        );
        assert_eq!(
            "en-".parse::<LanguageIdentifier>(),
            Err(LocaleParseError::InvalidSubtag)
        );
        assert_eq!(
            "en--US".parse::<LanguageIdentifier>(),
            Err(LocaleParseError::InvalidSubtag)
        );
        assert_eq!(
            "en-US-Latn".parse::<LanguageIdentifier>(),
            Err(LocaleParseError::InvalidSubtag)
        );
        assert_eq!(
            "en-US-DE".parse::<LanguageIdentifier>(),
            Err(LocaleParseError::InvalidSubtag)
        );
        assert_eq!(
            "en-posix-posix".parse::<LanguageIdentifier>(),
            Err(LocaleParseError::DuplicateVariant)
        );
        assert_eq!(
            "en-aaaaa-bbbbb-ccccc-ddddd-eeeee".parse::<LanguageIdentifier>(),
            Err(LocaleParseError::TooManyVariants)
        );
        assert!("en_US".parse::<LanguageIdentifier>().is_err());
        assert!("en-Ü".parse::<LanguageIdentifier>().is_err());
    }

    #[test]
    fn builders_replace_subtags() {
        let id = langid!("de-Latn-AT-1996");

        assert_eq!(id.with_region(None).to_string(), "de-Latn-1996");
        assert_eq!(id.with_script(None).to_string(), "de-AT-1996");
        assert_eq!(id.without_variants().to_string(), "de-Latn-AT");
        assert_eq!(
            id.with_variant(variant!("1901")).unwrap().to_string(),
            "de-Latn-AT-1901-1996"
        );
        assert_eq!(
            id.with_variant(variant!("1996")),
            Err(LocaleParseError::DuplicateVariant)
        );
    }

    #[test]
    fn unknown_identifier_is_the_unknown_language() {
        assert_eq!(LanguageIdentifier::UNKNOWN, parse("und"));
        assert!(LanguageIdentifier::UNKNOWN.language().is_unknown());
    }

    /// ICU parses the same texts to the same canonical identifiers, and the
    /// conversions in both directions preserve them.
    #[test]
    fn converts_to_and_from_icu() {
        for text in [
            "en",
            "EN-us",
            "zh-hant-tw",
            "es-419",
            "de-CH-1901",
            "sl-rozaj-biske-1994",
            "und",
        ] {
            let ours = parse(text);
            let icu = icu_locale_core::LanguageIdentifier::try_from_str(text).unwrap();

            assert_eq!(ours.to_icu(), icu, "{text}");
            assert_eq!(ours.to_icu().to_string(), ours.to_string(), "{text}");
            assert_eq!(LanguageIdentifier::try_from_icu(&icu), Ok(ours), "{text}");
        }
    }

    #[test]
    fn subtags_convert_to_and_from_icu() {
        assert_eq!(language!("de").to_icu().as_str(), "de");
        assert_eq!(script!("Hant").to_icu().as_str(), "Hant");
        assert_eq!(region!("419").to_icu().as_str(), "419");
        assert_eq!(variant!("1996").to_icu().as_str(), "1996");
        assert_eq!(
            Language::from_icu(icu_locale_core::subtags::Language::UNKNOWN),
            Language::UNKNOWN
        );
    }

    #[test]
    fn icu_identifiers_with_too_many_variants_are_rejected() {
        let icu =
            icu_locale_core::LanguageIdentifier::try_from_str("en-aaaaa-bbbbb-ccccc-ddddd-eeeee")
                .unwrap();

        assert_eq!(
            LanguageIdentifier::try_from_icu(&icu),
            Err(LocaleParseError::TooManyVariants)
        );
    }
}
