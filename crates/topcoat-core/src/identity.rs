//! Stable identities derived from a parent identity, a source location, and
//! an optional key.

mod key;
mod site;

use std::{fmt, str::FromStr};

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
pub use key::*;
pub use site::*;

use crate::fnv1a::Fnv1a;

/// Tag byte separating the parent hash from an unkeyed site.
const TAG_SITE: u8 = 0;
/// Tag byte separating the parent hash from a keyed site.
const TAG_KEYED: u8 = 1;

/// A stable identity derived from a chain of source locations and keys.
///
/// The same chain always derives the same identity, so an identity stays the
/// same across requests and renders. An identity can be ambiguous when it was
/// derived for repeated invocations that could not be told apart. A child
/// inherits the ambiguity of its parent. Deriving an ambiguous identity still
/// succeeds, and the error is only reported when the identity is read with
/// [`identity`](crate::context::identity) or
/// [`try_identity`](crate::context::try_identity).
///
/// An identity displays as 22 characters of URL-safe base64 and parses back
/// from that form with [`FromStr`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Identity {
    hash: u128,
    /// The label of the outermost ambiguous invocation on the chain, if any.
    ambiguity: Option<&'static str>,
}

impl Identity {
    /// The identity at the root of the tree.
    pub const ROOT: Self = Self {
        hash: 0,
        ambiguity: None,
    };

    /// Returns this identity, or an error if it is ambiguous.
    pub(crate) fn checked(self) -> Result<Self, AmbiguousIdentityError> {
        match self.ambiguity {
            None => Ok(self),
            Some(label) => Err(AmbiguousIdentityError { label }),
        }
    }

    /// Returns the hash value of this identity.
    #[must_use]
    pub const fn hash(self) -> u128 {
        self.hash
    }

    /// Derives the identity of a child invocation at `site`.
    ///
    /// An ambiguity on `self` carries over to the child.
    #[must_use]
    pub const fn child(self, site: SiteKey) -> Self {
        Self {
            hash: self.derive(TAG_SITE, site).finish(),
            ambiguity: self.ambiguity,
        }
    }

    /// Derives the identity of a keyed child invocation at `site`.
    ///
    /// The key tells repeated invocations at one site apart, so an invocation
    /// in a loop body can give each iteration its own identity. The same key
    /// at two different sites still derives two different identities. An
    /// ambiguity on `self` carries over to the child: a key only tells apart
    /// repetitions at its own site, not further up the chain.
    #[must_use]
    pub fn keyed_child(self, site: SiteKey, key: impl IdentityKey) -> Self {
        Self {
            hash: key
                .write(KeyHasher::new(self.derive(TAG_KEYED, site)))
                .finish(),
            ambiguity: self.ambiguity,
        }
    }

    /// Derives the identity of a child invocation at `site` whose
    /// repetitions cannot be told apart, recording `label` as the ambiguity.
    ///
    /// Derivation succeeds, but reading this identity or any identity derived
    /// from it reports the ambiguity. If the parent is already ambiguous, its
    /// label is kept.
    #[must_use]
    pub const fn ambiguous_child(self, site: SiteKey, label: &'static str) -> Self {
        Self {
            hash: self.derive(TAG_SITE, site).finish(),
            ambiguity: match self.ambiguity {
                Some(existing) => Some(existing),
                None => Some(label),
            },
        }
    }

    /// Starts a child derivation: the parent hash, a tag byte telling keyed
    /// and unkeyed derivations apart, then the site.
    ///
    /// Every segment is fixed-width, so no delimiters are needed; a keyed
    /// derivation folds the variable-width key in last.
    const fn derive(self, tag: u8, site: SiteKey) -> Fnv1a<u128> {
        Fnv1a::<u128>::new()
            .write(&self.hash.to_le_bytes())
            .write(&[tag])
            .write(&site.0.to_le_bytes())
    }
}

impl Default for Identity {
    fn default() -> Self {
        Self::ROOT
    }
}

/// Writes the identity's hash as 22 characters of URL-safe base64, the form
/// it takes on the wire.
impl fmt::Display for Identity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&URL_SAFE_NO_PAD.encode(self.hash.to_be_bytes()))
    }
}

/// Parses an identity from the form `Display` writes.
///
/// Use this to continue at an identity captured in an earlier request, for
/// example one a client sends back. The result is never ambiguous, because
/// only an identity that could be read was written out.
impl FromStr for Identity {
    type Err = ParseIdentityError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = URL_SAFE_NO_PAD.decode(s).map_err(|_| ParseIdentityError)?;
        let hash = <[u8; 16]>::try_from(bytes).map_err(|_| ParseIdentityError)?;
        Ok(Self {
            hash: u128::from_be_bytes(hash),
            ambiguity: None,
        })
    }
}

/// The error returned when parsing an [`Identity`] from a string that is not
/// in the form its `Display` impl writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseIdentityError;

impl fmt::Display for ParseIdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("expected an identity: 16 bytes of URL-safe base64 without padding")
    }
}

impl std::error::Error for ParseIdentityError {}

/// The error returned when reading an [`Identity`] that cannot tell repeated
/// scopes apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AmbiguousIdentityError {
    label: &'static str,
}

impl AmbiguousIdentityError {
    /// Returns the name of the scope that introduced the ambiguity.
    #[must_use]
    pub const fn label(&self) -> &'static str {
        self.label
    }
}

impl fmt::Display for AmbiguousIdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ambiguous identity: {} repeats without a key attribute; \
             add `#[key(...)]` to give each iteration its own identity",
            self.label,
        )
    }
}

impl std::error::Error for AmbiguousIdentityError {}

#[cfg(test)]
mod tests {
    use super::*;

    const SITE_A: SiteKey = SiteKey::new(file!(), line!(), column!(), 0);
    const SITE_B: SiteKey = SiteKey::new(file!(), line!(), column!(), 0);

    #[test]
    fn default_is_root() {
        assert_eq!(Identity::default(), Identity::ROOT);
        assert_eq!(Identity::ROOT.checked(), Ok(Identity::ROOT));
    }

    #[test]
    fn derivation_is_deterministic() {
        assert_eq!(Identity::ROOT.child(SITE_A), Identity::ROOT.child(SITE_A));
        assert_ne!(Identity::ROOT.child(SITE_A), Identity::ROOT.child(SITE_B));
        assert_ne!(Identity::ROOT.child(SITE_A), Identity::ROOT);
    }

    #[test]
    fn an_identity_round_trips_through_its_wire_form() {
        let identity = Identity::ROOT.child(SITE_A).keyed_child(SITE_B, 3);
        let wire = identity.to_string();
        assert_eq!(wire.len(), 22);
        assert!(
            wire.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        );
        assert_eq!(wire.parse::<Identity>().unwrap(), identity);
        assert_eq!(
            wire.parse::<Identity>().unwrap().child(SITE_A),
            identity.child(SITE_A),
        );
        assert_eq!(
            Identity::ROOT.to_string().parse::<Identity>().unwrap(),
            Identity::ROOT
        );
        assert_eq!("not base64".parse::<Identity>(), Err(ParseIdentityError));
        assert_eq!("AAAA".parse::<Identity>(), Err(ParseIdentityError));
    }

    #[test]
    fn keys_tell_repetitions_of_one_site_apart() {
        let root = Identity::ROOT;
        assert_eq!(root.keyed_child(SITE_A, 1), root.keyed_child(SITE_A, 1));
        assert_ne!(root.keyed_child(SITE_A, 1), root.keyed_child(SITE_A, 2));
    }

    #[test]
    fn the_site_stays_mixed_into_a_keyed_identity() {
        let root = Identity::ROOT;
        assert_ne!(root.keyed_child(SITE_A, 1), root.keyed_child(SITE_B, 1));
    }

    #[test]
    fn keyed_and_unkeyed_children_never_collide() {
        let root = Identity::ROOT;
        assert_ne!(root.child(SITE_A), root.keyed_child(SITE_A, ""));
    }

    #[test]
    fn ambiguity_poisons_keyed_descendants() {
        let poisoned = Identity::ROOT.ambiguous_child(SITE_A, "outer");
        assert_eq!(poisoned.keyed_child(SITE_B, 7).ambiguity, Some("outer"));
        assert_eq!(poisoned.child(SITE_B).ambiguity, Some("outer"));
    }

    #[test]
    fn the_outermost_ambiguity_wins() {
        let poisoned = Identity::ROOT
            .ambiguous_child(SITE_A, "outer")
            .ambiguous_child(SITE_B, "inner");
        assert_eq!(poisoned.ambiguity, Some("outer"));
    }
}
