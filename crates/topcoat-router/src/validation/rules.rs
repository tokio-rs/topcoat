//! Small predicates for the valid case of a field.
//!
//! Pair them with [`ValidationErrors::check`](super::ValidationErrors::check) so
//! each rule reads as the condition that must hold. Lengths count characters,
//! not bytes, so multibyte input is measured the way users see it.
//!
//! ```rust
//! use topcoat::validation::{ValidationErrors, rules};
//!
//! let mut errors = ValidationErrors::new();
//! errors.check(
//!     "title",
//!     rules::required("  hello  "),
//!     "title cannot be empty",
//! );
//! assert!(errors.is_empty());
//! ```
//!
//! Each rule has a canonical machine-readable code for
//! [`check_with_code`](super::ValidationErrors::check_with_code), so clients
//! can match on failures across endpoints. The constants below pin those
//! strings as part of the API contract.

/// The code for a missing value. Pairs with [`required`].
pub const CODE_REQUIRED: &str = "required";

/// The code for input shorter than the minimum. Pairs with [`min_length`].
pub const CODE_TOO_SHORT: &str = "too_short";

/// The code for input longer than the maximum. Pairs with [`max_length`].
pub const CODE_TOO_LONG: &str = "too_long";

/// The code for a number outside its bounds. Pairs with [`range`].
pub const CODE_OUT_OF_RANGE: &str = "out_of_range";

/// The code for a malformed address. Pairs with [`email`].
pub const CODE_INVALID_EMAIL: &str = "invalid_email";

/// Returns whether `value` holds non-blank text after trimming.
///
/// Report failures with [`CODE_REQUIRED`].
#[must_use]
pub fn required(value: &str) -> bool {
    !value.trim().is_empty()
}

/// Returns whether `value` holds between `min` and `max` characters inclusive.
///
/// Prefer [`min_length`] and [`max_length`] when reporting codes: one bound
/// maps to [`CODE_TOO_SHORT`], the other to [`CODE_TOO_LONG`], while this
/// combined check cannot tell which side failed.
#[must_use]
pub fn length(value: &str, min: usize, max: usize) -> bool {
    let len = value.chars().count();
    len >= min && len <= max
}

/// Returns whether `value` holds at least `min` characters.
///
/// Report failures with [`CODE_TOO_SHORT`].
#[must_use]
pub fn min_length(value: &str, min: usize) -> bool {
    value.chars().count() >= min
}

/// Returns whether `value` holds at most `max` characters.
///
/// Report failures with [`CODE_TOO_LONG`].
#[must_use]
pub fn max_length(value: &str, max: usize) -> bool {
    value.chars().count() <= max
}

/// Returns whether `value` lies between `min` and `max` inclusive.
///
/// Report failures with [`CODE_OUT_OF_RANGE`].
#[must_use]
pub fn range<T: PartialOrd + Copy>(value: T, min: T, max: T) -> bool {
    min <= value && value <= max
}

/// Returns whether `value` looks like an email address.
///
/// Leading and trailing whitespace is ignored for the check; the input is
/// not modified. This is a fast heuristic (one `@`, a non-empty local part,
/// a dotted domain without spaces or empty labels), not an RFC parse. Use a
/// dedicated address parser when deliverability must be proven. Report
/// failures with [`CODE_INVALID_EMAIL`].
#[must_use]
pub fn email(value: &str) -> bool {
    let value = value.trim();
    if value.contains(' ') {
        return false;
    }

    let mut parts = value.split('@');
    let (Some(local), Some(domain), None) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };

    if local.is_empty() || domain.is_empty() {
        return false;
    }

    if !domain.contains('.') {
        return false;
    }

    if domain.split('.').any(str::is_empty) {
        return false;
    }

    domain.rsplit('.').next().is_some_and(|tld| tld.len() >= 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_codes_are_stable() {
        assert_eq!(CODE_REQUIRED, "required");
        assert_eq!(CODE_TOO_SHORT, "too_short");
        assert_eq!(CODE_TOO_LONG, "too_long");
        assert_eq!(CODE_OUT_OF_RANGE, "out_of_range");
        assert_eq!(CODE_INVALID_EMAIL, "invalid_email");
    }

    #[test]
    fn required_trims_whitespace() {
        assert!(required("ada"));
        assert!(required("  ada  "));
        assert!(!required(""));
        assert!(!required("   "));
    }

    #[test]
    fn length_counts_characters_not_bytes() {
        assert!(length("hello", 1, 5));
        assert!(!length("", 1, 5));
        assert!(!length("hello!", 1, 5));
        assert!(length("h\u{e9}llo", 5, 5));
        assert!(min_length("hello", 5));
        assert!(!min_length("hi", 5));
        assert!(max_length("hi", 5));
        assert!(!max_length("hello!", 5));
    }

    #[test]
    fn range_is_inclusive() {
        assert!(range(1, 1, 3));
        assert!(range(3, 1, 3));
        assert!(!range(0, 1, 3));
        assert!(!range(4, 1, 3));
    }

    #[test]
    fn email_accepts_ordinary_addresses() {
        assert!(email("ada@example.com"));
        assert!(email("ada.lovelace@exa-mple.co"));
        assert!(email("  ada@example.com "));
    }

    #[test]
    fn email_rejects_malformed_addresses() {
        for value in [
            "",
            "ada",
            "ada@",
            "@example.com",
            "ada@example",
            "ada@ex..com",
            "ada@.com",
            "a@b.c",
            "a b@example.com",
            "ada@@example.com",
        ] {
            assert!(!email(value), "{value} is not an email");
        }
    }
}
