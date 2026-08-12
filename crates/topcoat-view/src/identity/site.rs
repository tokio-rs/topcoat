use topcoat_core::fnv1a::Fnv1a;

/// A compile-time key for one component invocation site.
///
/// The `view!` macro builds one per component invocation, as
/// `const { SiteKey::new(file!(), line!(), column!(), ordinal) }`. The
/// macro's spans all resolve to the position of the invocation itself, so
/// `file`, `line`, and `column` alone cannot tell two components in one
/// macro body apart; the `ordinal` numbers them in emission order.
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
}
