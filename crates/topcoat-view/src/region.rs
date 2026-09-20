use std::fmt;

use topcoat_core::identity::{Identity, SiteKey};

/// Identifies a region by its enclosing identity and source location.
///
/// The same region has the same id across renders. Its wire form is the
/// 128-bit hash written as fixed-width hex.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RegionId(u128);

impl RegionId {
    /// Derives a region id at `site` below `identity`.
    pub(crate) const fn new(identity: Identity, site: SiteKey) -> Self {
        Self(identity.child(site).hash())
    }
}

impl fmt::Display for RegionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:032x}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SITE_A: SiteKey = SiteKey::new(file!(), line!(), column!(), 0);
    const SITE_B: SiteKey = SiteKey::new(file!(), line!(), column!(), 0);

    #[test]
    fn a_region_depends_on_its_identity_and_site() {
        let id = RegionId::new(Identity::ROOT, SITE_A);
        assert_eq!(id, RegionId::new(Identity::ROOT, SITE_A));
        assert_ne!(id, RegionId::new(Identity::ROOT, SITE_B));
        assert_ne!(id, RegionId::new(Identity::ROOT.child(SITE_B), SITE_A));
    }

    #[test]
    fn an_identity_restored_from_the_wire_derives_the_same_region() {
        let identity = Identity::ROOT.keyed_child(SITE_A, "item");
        let restored = identity.to_string().parse().unwrap();
        assert_eq!(
            RegionId::new(identity, SITE_B),
            RegionId::new(restored, SITE_B),
        );
    }

    #[test]
    fn the_wire_form_is_fixed_width_hex() {
        assert_eq!(RegionId(1).to_string(), "00000000000000000000000000000001");
        assert_eq!(
            RegionId(u128::MAX).to_string(),
            "ffffffffffffffffffffffffffffffff",
        );
    }
}
