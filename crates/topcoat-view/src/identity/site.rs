use std::panic::Location;

use topcoat_core::fnv1a::Fnv1a;

/// A key for one source location that derives an identity.
///
/// The `view!` macro builds one per component invocation, as
/// `const { SiteKey::new(file!(), line!(), column!(), ordinal) }`. The
/// macro's spans all resolve to the position of the invocation itself, so
/// `file`, `line`, and `column` alone cannot tell two components in one
/// macro body apart; the `ordinal` numbers them in emission order.
///
/// A function that derives an identity for its caller builds one from the
/// caller's [`Location`] with [`from_location`](Self::from_location)
/// instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SiteKey(pub(super) u64);

impl SiteKey {
    /// Creates a site key from a source location and an ordinal.
    ///
    /// `const`, so the hashing happens at compile time when the inputs are
    /// literals from `file!`, `line!`, and `column!`.
    #[must_use]
    pub const fn new(file: &str, line: u32, column: u32, ordinal: u32) -> Self {
        Self(
            Fnv1a::<u64>::new()
                .write(file.as_bytes())
                .write(b"\0")
                .write(&line.to_le_bytes())
                .write(&column.to_le_bytes())
                .write(&ordinal.to_le_bytes())
                .finish(),
        )
    }

    /// Creates a site key from a runtime source location, such as the one
    /// `Location::caller` reports inside a `#[track_caller]` function.
    ///
    /// The ordinal is zero: a call expression has one position in source.
    #[must_use]
    pub const fn from_location(location: &Location<'_>) -> Self {
        Self::new(location.file(), location.line(), location.column(), 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn site_keys_at_distinct_locations_differ() {
        let base = SiteKey::new("src/a.rs", 1, 1, 0);
        assert_ne!(base, SiteKey::new("src/b.rs", 1, 1, 0));
        assert_ne!(base, SiteKey::new("src/a.rs", 2, 1, 0));
        assert_ne!(base, SiteKey::new("src/a.rs", 1, 2, 0));
        assert_ne!(base, SiteKey::new("src/a.rs", 1, 1, 1));
    }

    #[test]
    fn a_location_keys_like_its_coordinates() {
        let location = Location::caller();
        assert_eq!(
            SiteKey::from_location(location),
            SiteKey::new(location.file(), location.line(), location.column(), 0),
        );
    }
}
