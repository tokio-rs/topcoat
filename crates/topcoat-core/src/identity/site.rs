use std::panic::Location;

use xxhash_rust::{const_xxh3, xxh3};

/// A key for one source location that derives an identity.
///
/// Generated code can combine `file!()`, `line!()`, and `column!()` with an
/// ordinal to distinguish multiple sites within one macro invocation.
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
        let h = const_xxh3::xxh3_64_with_seed(file.as_bytes(), 0);
        let h = const_xxh3::xxh3_64_with_seed(&line.to_le_bytes(), h);
        let h = const_xxh3::xxh3_64_with_seed(&column.to_le_bytes(), h);
        Self(const_xxh3::xxh3_64_with_seed(&ordinal.to_le_bytes(), h))
    }

    /// Creates a site key from a runtime source location, such as the one
    /// `Location::caller` reports inside a `#[track_caller]` function.
    ///
    /// The ordinal is zero: a call expression has one position in source.
    /// The key equals the one [`new`](Self::new) creates from the same
    /// coordinates.
    #[must_use]
    pub fn from_location(location: &Location<'_>) -> Self {
        let h = xxh3::xxh3_64_with_seed(location.file().as_bytes(), 0);
        let h = xxh3::xxh3_64_with_seed(&location.line().to_le_bytes(), h);
        let h = xxh3::xxh3_64_with_seed(&location.column().to_le_bytes(), h);
        Self(xxh3::xxh3_64_with_seed(&0u32.to_le_bytes(), h))
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
