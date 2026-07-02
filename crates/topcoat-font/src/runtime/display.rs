//! Font display strategies for the `font-display` descriptor on `@font-face`
//! rules.

/// How a font face is displayed while it loads, as named by the CSS
/// `font-display` descriptor of an `@font-face` rule.
///
/// The strategy governs the *block* period (during which text renders
/// invisibly, awaiting the face) and the *swap* period (during which a fallback
/// renders but is swapped for the face once it loads). [`FontDisplay::default`]
/// is [`Auto`](FontDisplay::Auto), matching CSS.
///
/// Displays as the CSS keyword (`auto`, `block`, `swap`, `fallback`,
/// `optional`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FontDisplay {
    /// The font display strategy is defined by the user agent, CSS `auto`.
    #[default]
    Auto,
    /// Gives the font face a short block period and an infinite swap period,
    /// CSS `block`.
    Block,
    /// Gives the font face an extremely small block period and an infinite swap
    /// period, CSS `swap`.
    Swap,
    /// Gives the font face an extremely small block period and a short swap
    /// period, CSS `fallback`.
    Fallback,
    /// Gives the font face an extremely small block period and no swap period,
    /// CSS `optional`.
    Optional,
}

impl FontDisplay {
    /// The CSS keyword for this strategy, as written for the `font-display`
    /// descriptor.
    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Block => "block",
            Self::Swap => "swap",
            Self::Fallback => "fallback",
            Self::Optional => "optional",
        }
    }

    /// The strategy named by the given CSS `font-display` keyword, if the
    /// keyword names a known strategy.
    #[must_use]
    pub fn from_keyword(keyword: &str) -> Option<Self> {
        Some(match keyword {
            "auto" => Self::Auto,
            "block" => Self::Block,
            "swap" => Self::Swap,
            "fallback" => Self::Fallback,
            "optional" => Self::Optional,
            _ => return None,
        })
    }

    /// Folds this strategy into a running content hash.
    pub(crate) const fn hash(self, h: u64) -> u64 {
        topcoat_core::runtime::fnv1a::hash_continue(h, self.keyword().as_bytes())
    }
}

impl std::fmt::Display for FontDisplay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.keyword())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_auto() {
        assert_eq!(FontDisplay::default(), FontDisplay::Auto);
    }

    #[test]
    fn displays_as_its_keyword() {
        assert_eq!(FontDisplay::Auto.to_string(), "auto");
        assert_eq!(FontDisplay::Block.to_string(), "block");
        assert_eq!(FontDisplay::Swap.to_string(), "swap");
        assert_eq!(FontDisplay::Fallback.to_string(), "fallback");
        assert_eq!(FontDisplay::Optional.to_string(), "optional");
    }

    #[test]
    fn from_keyword_round_trips() {
        for strategy in [
            FontDisplay::Auto,
            FontDisplay::Block,
            FontDisplay::Swap,
            FontDisplay::Fallback,
            FontDisplay::Optional,
        ] {
            assert_eq!(
                FontDisplay::from_keyword(strategy.keyword()),
                Some(strategy)
            );
        }
    }

    #[test]
    fn from_keyword_rejects_unknown() {
        assert_eq!(FontDisplay::from_keyword("infinite"), None);
    }
}
