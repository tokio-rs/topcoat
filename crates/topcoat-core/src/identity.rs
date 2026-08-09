//! Invocation identity: a stable id for each invocation in a tree of
//! nested calls, derived from the chain of call sites leading down to it.
//!
//! An [`Identity`] is a 128-bit hash mixing the identity of the enclosing
//! invocation with a [`SiteKey`] naming the invocation's location in
//! source. Because derivation depends only on call sites, an identity is
//! reproducible: the same invocation reached through the same chain of
//! call sites hashes to the same value. This is what keeps a component's
//! id stable from one render to the next, but nothing here is specific to
//! components.
//!
//! The current identity travels down the tree through a thread local
//! installed for exactly the duration of an invocation. [`IdentityGuard`]
//! installs one around a synchronous region, and [`IdentityFuture`] around
//! every poll of a future, so sibling futures interleaving on one task
//! each see their own identity. [`Identity::current`] reads the installed
//! identity from inside an invocation.
//!
//! An invocation that repeats, for example inside a `for` body, shares one
//! call site across all repetitions. A `key` argument mixes a caller-provided
//! [`IdentityKey`] value into the identity to tell the repetitions apart.
//! Without one the identity is ambiguous: derivation still succeeds and
//! execution proceeds, but the ambiguity is recorded, poisons every identity
//! derived below it, and [`Identity::current`] panics if a descendant
//! actually consumes the identity, naming the invocation that is missing its
//! `key`.

mod future;
mod guard;
mod key;
mod site;

use std::{cell::Cell, fmt};

pub use future::*;
pub use guard::*;
pub use key::*;
pub use site::*;

use crate::fnv1a::Fnv1a;

thread_local! {
    /// The identity of the invocation running on the current thread, if
    /// any.
    ///
    /// [`IdentityGuard`] installs an identity here for exactly the duration
    /// of a synchronous region, and [`IdentityFuture`] for exactly the
    /// duration of each of its polls, so futures that interleave on one task
    /// never see each other's identity. An empty cell means the root: no
    /// invocation is running.
    static CURRENT: Cell<Option<Identity>> = const { Cell::new(None) };
}

/// Tag byte separating the parent hash from an unkeyed site.
const TAG_SITE: u8 = 0;
/// Tag byte separating the parent hash from a keyed site.
const TAG_KEYED: u8 = 1;

/// The identity of an invocation: a hash of the chain of call sites from
/// the root of the tree down to it.
///
/// Identities form a tree. [`child`](Self::child) and
/// [`keyed_child`](Self::keyed_child) derive the identity one level down,
/// and [`current`](Self::current) reads the identity installed for the
/// running invocation. An identity is a plain `Copy` value; holding one
/// does not keep anything installed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Identity {
    hash: u128,
    /// The label of the outermost ambiguous invocation on the chain, if any.
    ambiguity: Option<&'static str>,
}

impl Identity {
    /// The identity at the root of the tree, outside any invocation.
    pub const ROOT: Self = Self {
        hash: 0,
        ambiguity: None,
    };

    /// Returns the identity of the running invocation.
    ///
    /// Outside any invocation this is [`ROOT`](Self::ROOT).
    ///
    /// # Panics
    ///
    /// Panics if the identity is ambiguous, meaning an invocation on the
    /// chain repeats without a `key` argument. The message names that
    /// invocation. Consumers that can work without an identity use
    /// [`try_current`](Self::try_current) instead.
    #[must_use]
    #[track_caller]
    pub fn current() -> Self {
        match Self::try_current() {
            Ok(identity) => identity,
            Err(error) => panic!("{error}"),
        }
    }

    /// Returns the identity of the running invocation, or the ambiguity
    /// poisoning it.
    ///
    /// The tolerant counterpart of [`current`](Self::current), for consumers
    /// that can fall back to working without an identity.
    ///
    /// # Errors
    ///
    /// Errors if the identity is ambiguous, meaning an invocation on the
    /// chain repeats without a `key` argument. The error names that
    /// invocation.
    pub fn try_current() -> Result<Self, AmbiguousIdentityError> {
        let identity = Self::current_raw();
        match identity.ambiguity {
            None => Ok(identity),
            Some(label) => Err(AmbiguousIdentityError { label }),
        }
    }

    /// Reads the installed identity without checking for ambiguity, falling
    /// back to [`ROOT`](Self::ROOT) when none is installed.
    fn current_raw() -> Self {
        CURRENT.get().unwrap_or(Self::ROOT)
    }

    /// The hash value of this identity.
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
    /// The key tells repetitions of one invocation site apart, so an
    /// invocation in a loop body can give each iteration its own identity.
    /// The site stays mixed in: the same key at two different sites still
    /// derives two different identities. An ambiguity on `self` carries over
    /// to the child; a key resolves repetition at its own site, not on the
    /// chain above it.
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
    /// Meant for an invocation that sits in a loop body without a `key`
    /// argument; `label` names that invocation. The hash is still derived
    /// and execution proceeds, but the ambiguity poisons this identity and
    /// every identity derived from it, keyed or not, so consuming one
    /// through [`current`](Self::current) panics. An ambiguity already on
    /// `self` wins: the outermost missing key is the one to fix first.
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

/// Error returned by [`Identity::try_current`] when the identity is
/// poisoned by an invocation that repeats without a `key` argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AmbiguousIdentityError {
    label: &'static str,
}

impl AmbiguousIdentityError {
    /// Names the invocation that is missing its `key` argument.
    #[must_use]
    pub const fn label(&self) -> &'static str {
        self.label
    }
}

impl fmt::Display for AmbiguousIdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ambiguous identity: {} repeats without a `key` argument; \
             pass `key:` to give each repetition its own identity",
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
    fn current_is_root_outside_any_invocation() {
        assert_eq!(Identity::current(), Identity::ROOT);
        assert_eq!(Identity::try_current(), Ok(Identity::ROOT));
    }

    #[test]
    fn derivation_is_deterministic() {
        assert_eq!(Identity::ROOT.child(SITE_A), Identity::ROOT.child(SITE_A));
        assert_ne!(Identity::ROOT.child(SITE_A), Identity::ROOT.child(SITE_B));
        assert_ne!(Identity::ROOT.child(SITE_A), Identity::ROOT);
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
