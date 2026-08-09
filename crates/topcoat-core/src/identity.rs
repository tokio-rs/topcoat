//! Component identity: a stable id for each component invocation, derived
//! from the chain of call sites leading down to it.
//!
//! An [`Identity`] is a 128-bit hash mixing the identity of the enclosing
//! component body with a [`SiteKey`] naming the invocation's location in
//! source. Because derivation depends only on where a component is invoked,
//! an identity is stable across renders: the same invocation reached through
//! the same chain of call sites hashes to the same value.
//!
//! The current identity travels down the tree through a thread local
//! installed for exactly the duration of a component body. [`IdentityGuard`]
//! installs one around a synchronous region, and [`IdentityFuture`] around
//! every poll of a render future, so sibling futures interleaving on one
//! task each see their own identity. [`Identity::current`] reads the
//! installed identity from inside a component body.
//!
//! An invocation that repeats, for example inside a `for` body, shares one
//! call site across all repetitions. A `key` argument mixes a caller-provided
//! value into the identity to tell the repetitions apart. Without one the
//! identity is ambiguous: derivation still succeeds and rendering proceeds,
//! but the ambiguity is recorded, poisons every identity derived below it,
//! and [`Identity::current`] panics if a descendant actually consumes the
//! identity, naming the invocation that is missing its `key`.

use std::{
    cell::Cell,
    fmt::{self, Write},
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use pin_project_lite::pin_project;

use crate::fnv1a::Fnv1a;

thread_local! {
    /// The identity of the component body running on the current thread, if
    /// any.
    ///
    /// [`IdentityGuard`] installs an identity here for exactly the duration
    /// of a synchronous region, and [`IdentityFuture`] for exactly the
    /// duration of each of its polls, so futures that interleave on one task
    /// never see each other's identity. An empty cell means the root: no
    /// component body is running.
    static CURRENT: Cell<Option<Identity>> = const { Cell::new(None) };
}

/// A compile-time key for one component invocation site.
///
/// The `view!` macro builds one per component invocation, as
/// `const { SiteKey::new(file!(), line!(), column!(), ordinal) }`. The
/// macro's spans all resolve to the position of the invocation itself, so
/// `file`, `line`, and `column` alone cannot tell two components in one
/// macro body apart; the `ordinal` numbers them in emission order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SiteKey(u64);

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

/// Tag byte separating the parent hash from an unkeyed site.
const TAG_SITE: u8 = 0;
/// Tag byte separating the parent hash from a keyed site.
const TAG_KEYED: u8 = 1;

/// The identity of a component invocation: a hash of the chain of call
/// sites from the root of the tree down to it.
///
/// Identities form a tree. [`child`](Self::child) and
/// [`keyed_child`](Self::keyed_child) derive the identity one level down,
/// and [`current`](Self::current) reads the identity installed for the
/// running component body. An identity is a plain `Copy` value; holding one
/// does not keep anything installed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Identity {
    hash: u128,
    /// The label of the outermost ambiguous invocation on the chain, if any.
    ambiguity: Option<&'static str>,
}

impl Identity {
    /// The identity at the root of the tree, outside any component body.
    pub const ROOT: Self = Self {
        hash: 0,
        ambiguity: None,
    };

    /// Returns the identity of the running component body.
    ///
    /// Outside any component body this is [`ROOT`](Self::ROOT).
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

    /// Returns the identity of the running component body, or the ambiguity
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
    /// Keys hash by their `Display` output, streamed without allocating.
    /// The site stays mixed in: the same key at two different sites still
    /// derives two different identities. An ambiguity on `self` carries over
    /// to the child; a key resolves repetition at its own site, not on the
    /// chain above it.
    ///
    /// # Panics
    ///
    /// Panics if the key's `Display` implementation returns an error.
    #[must_use]
    pub fn keyed_child(self, site: SiteKey, key: impl fmt::Display) -> Self {
        let mut hasher = DisplayHasher(self.derive(TAG_KEYED, site));
        write!(hasher, "{key}").expect("the key's Display impl returned an error");
        Self {
            hash: hasher.0.finish(),
            ambiguity: self.ambiguity,
        }
    }

    /// Derives the identity of a child invocation at `site` whose
    /// repetitions cannot be told apart, recording `label` as the ambiguity.
    ///
    /// The `view!` macro derives this for an invocation that sits in a loop
    /// body without a `key` argument; `label` names that invocation. The
    /// hash is still derived and rendering proceeds, but the ambiguity
    /// poisons this identity and every identity derived from it, keyed or
    /// not, so consuming one through [`current`](Self::current) panics. An
    /// ambiguity already on `self` wins: the outermost missing key is the
    /// one to fix first.
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
    /// derivation appends the variable-width key text last.
    const fn derive(self, tag: u8, site: SiteKey) -> Fnv1a<u128> {
        Fnv1a::<u128>::new()
            .write(&self.hash.to_le_bytes())
            .write(&[tag])
            .write(&site.0.to_le_bytes())
    }
}

/// Feeds a key's `Display` output into a running hash without allocating.
///
/// The hasher moves through every `write`, so each chunk of formatted
/// output swaps a fresh hasher in to move the running one out and swaps the
/// advanced one back.
struct DisplayHasher(Fnv1a<u128>);

impl Write for DisplayHasher {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let hasher = std::mem::take(&mut self.0);
        self.0 = hasher.write(s.as_bytes());
        Ok(())
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
            "ambiguous component identity: {} repeats without a `key` argument; \
             pass `key:` to give each repetition its own identity",
            self.label,
        )
    }
}

impl std::error::Error for AmbiguousIdentityError {}

/// Installs an identity for exactly the duration of a synchronous region.
///
/// Creating a guard derives the identity one level down from the installed
/// one and swaps it in; dropping the guard swaps the previous identity
/// back, also when the region panics. The guard is the synchronous
/// counterpart of [`IdentityFuture`], for component bodies that build in a
/// single burst.
#[must_use = "the identity is uninstalled when the guard drops"]
pub struct IdentityGuard {
    prev: Option<Identity>,
    /// The guard restores a thread local, so it must stay on the thread it
    /// was created on.
    _not_send: PhantomData<*const ()>,
}

impl IdentityGuard {
    /// Enters a child invocation at `site`.
    pub fn enter(site: SiteKey) -> Self {
        Self::install(Identity::current_raw().child(site))
    }

    /// Enters a keyed child invocation at `site`.
    pub fn enter_keyed(site: SiteKey, key: impl fmt::Display) -> Self {
        Self::install(Identity::current_raw().keyed_child(site, key))
    }

    /// Enters a child invocation at `site` whose repetitions cannot be told
    /// apart, recording `label` as the ambiguity.
    pub fn enter_ambiguous(site: SiteKey, label: &'static str) -> Self {
        Self::install(Identity::current_raw().ambiguous_child(site, label))
    }

    /// Installs `identity` verbatim, without deriving from the installed
    /// one.
    ///
    /// The door for re-entering a subtree at a known identity, for example
    /// when resuming an isolated render at an identity captured earlier.
    pub fn install(identity: Identity) -> Self {
        Self {
            prev: CURRENT.replace(Some(identity)),
            _not_send: PhantomData,
        }
    }
}

impl Drop for IdentityGuard {
    fn drop(&mut self) {
        CURRENT.set(self.prev);
    }
}

pin_project! {
    /// Installs an identity around every poll of a component's render
    /// future.
    ///
    /// The identity is derived once, at construction, from the identity
    /// installed at that moment: construction happens inside the parent's
    /// body, so the parent is captured even though the future may be polled
    /// later, interleaved with its siblings. Each poll then installs the
    /// derived identity for exactly its duration, so siblings running
    /// concurrently on one task each see their own.
    #[must_use = "futures do nothing unless polled"]
    pub struct IdentityFuture<F> {
        #[pin]
        fut: F,
        identity: Identity,
    }
}

impl<F> IdentityFuture<F> {
    /// Wraps `fut` as a child invocation at `site`.
    pub fn new(site: SiteKey, fut: F) -> Self {
        Self {
            fut,
            identity: Identity::current_raw().child(site),
        }
    }

    /// Wraps `fut` as a keyed child invocation at `site`.
    pub fn keyed(site: SiteKey, key: impl fmt::Display, fut: F) -> Self {
        Self {
            fut,
            identity: Identity::current_raw().keyed_child(site, key),
        }
    }

    /// Wraps `fut` as a child invocation at `site` whose repetitions cannot
    /// be told apart, recording `label` as the ambiguity.
    pub fn ambiguous(site: SiteKey, label: &'static str, fut: F) -> Self {
        Self {
            fut,
            identity: Identity::current_raw().ambiguous_child(site, label),
        }
    }
}

impl<F: Future> Future for IdentityFuture<F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, task_cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let _guard = IdentityGuard::install(*this.identity);
        this.fut.poll(task_cx)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        panic::{AssertUnwindSafe, catch_unwind},
        pin::pin,
        task::Waker,
    };

    use super::*;

    const SITE_A: SiteKey = SiteKey::new(file!(), line!(), column!(), 0);
    const SITE_B: SiteKey = SiteKey::new(file!(), line!(), column!(), 0);

    /// Drives `fut` to completion on the current thread.
    ///
    /// The futures under test never wait on external events, so polling in
    /// a tight loop is sufficient.
    fn block_on<F: Future>(fut: F) -> F::Output {
        let mut fut = pin!(fut);
        let mut task = Context::from_waker(Waker::noop());
        loop {
            if let Poll::Ready(output) = fut.as_mut().poll(&mut task) {
                return output;
            }
        }
    }

    /// A future that is pending on its first poll and ready on its second.
    struct YieldOnce(bool);

    impl Future for YieldOnce {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, _task_cx: &mut Context<'_>) -> Poll<()> {
            if self.0 {
                Poll::Ready(())
            } else {
                self.0 = true;
                Poll::Pending
            }
        }
    }

    #[test]
    fn current_is_root_outside_any_component() {
        assert_eq!(Identity::current(), Identity::ROOT);
        assert_eq!(Identity::try_current(), Ok(Identity::ROOT));
    }

    #[test]
    fn site_keys_at_distinct_locations_differ() {
        let base = SiteKey::new("src/a.rs", 1, 1, 0);
        assert_ne!(base, SiteKey::new("src/b.rs", 1, 1, 0));
        assert_ne!(base, SiteKey::new("src/a.rs", 2, 1, 0));
        assert_ne!(base, SiteKey::new("src/a.rs", 1, 2, 0));
        assert_ne!(base, SiteKey::new("src/a.rs", 1, 1, 1));
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
    fn keys_hash_by_their_display_output() {
        let root = Identity::ROOT;
        assert_eq!(root.keyed_child(SITE_A, 1), root.keyed_child(SITE_A, "1"));
    }

    #[test]
    fn guards_install_and_restore() {
        {
            let _guard = IdentityGuard::enter(SITE_A);
            assert_eq!(Identity::current(), Identity::ROOT.child(SITE_A));
            {
                let _inner = IdentityGuard::enter_keyed(SITE_B, 7);
                assert_eq!(
                    Identity::current(),
                    Identity::ROOT.child(SITE_A).keyed_child(SITE_B, 7),
                );
            }
            assert_eq!(Identity::current(), Identity::ROOT.child(SITE_A));
        }
        assert_eq!(Identity::current(), Identity::ROOT);
    }

    #[test]
    fn guards_restore_when_the_region_panics() {
        let result = catch_unwind(|| {
            let _guard = IdentityGuard::enter(SITE_A);
            panic!("boom");
        });
        assert!(result.is_err());
        assert_eq!(Identity::current(), Identity::ROOT);
    }

    #[test]
    fn an_ambiguous_identity_errors_on_consumption() {
        let _guard = IdentityGuard::enter_ambiguous(SITE_A, "`card` at src/a.rs:1");
        let error = Identity::try_current().unwrap_err();
        assert_eq!(error.label(), "`card` at src/a.rs:1");

        let panic = catch_unwind(|| Identity::current()).unwrap_err();
        let message = panic.downcast::<String>().expect("panics with a message");
        assert!(message.contains("`card` at src/a.rs:1"));
        assert!(message.contains("`key`"));
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

    #[test]
    fn the_future_installs_its_identity_only_while_polling() {
        let fut = IdentityFuture::new(SITE_A, async {
            assert_eq!(Identity::current(), Identity::ROOT.child(SITE_A));
        });
        assert_eq!(Identity::current(), Identity::ROOT);
        block_on(fut);
        assert_eq!(Identity::current(), Identity::ROOT);
    }

    #[test]
    fn the_identity_is_derived_at_construction() {
        let fut = {
            let _parent = IdentityGuard::enter(SITE_A);
            IdentityFuture::new(SITE_B, async { Identity::current() })
        };
        // The parent guard is gone, but the future captured its identity.
        assert_eq!(block_on(fut), Identity::ROOT.child(SITE_A).child(SITE_B),);
    }

    #[test]
    fn interleaved_siblings_each_see_their_own_identity() {
        let sibling = |key: u32| {
            IdentityFuture::keyed(SITE_A, key, async move {
                let before = Identity::current();
                YieldOnce(false).await;
                assert_eq!(Identity::current(), before);
                before
            })
        };
        let mut first = pin!(sibling(1));
        let mut second = pin!(sibling(2));
        let mut task = Context::from_waker(Waker::noop());

        // Interleave the two futures across their yield points.
        assert!(first.as_mut().poll(&mut task).is_pending());
        assert!(second.as_mut().poll(&mut task).is_pending());
        let Poll::Ready(first) = first.as_mut().poll(&mut task) else {
            panic!("ready on the second poll");
        };
        let Poll::Ready(second) = second.as_mut().poll(&mut task) else {
            panic!("ready on the second poll");
        };
        assert_ne!(first, second);
        assert_eq!(first, Identity::ROOT.keyed_child(SITE_A, 1));
        assert_eq!(second, Identity::ROOT.keyed_child(SITE_A, 2));
    }

    #[test]
    fn the_future_restores_the_identity_when_a_poll_panics() {
        let mut fut = pin!(IdentityFuture::new(SITE_A, async { panic!("boom") }));
        let result = catch_unwind(AssertUnwindSafe(|| {
            let mut task = Context::from_waker(Waker::noop());
            let _ = fut.as_mut().poll(&mut task);
        }));
        assert!(result.is_err());
        assert_eq!(Identity::current(), Identity::ROOT);
    }

    #[test]
    fn an_ambiguous_future_poisons_its_descendants() {
        let fut = IdentityFuture::ambiguous(SITE_A, "`card` at src/a.rs:1", async {
            let error = Identity::try_current().unwrap_err();
            let keyed =
                IdentityFuture::keyed(SITE_B, 7, async { Identity::try_current().unwrap_err() });
            (error, keyed.await)
        });
        let (outer, inner) = block_on(fut);
        assert_eq!(outer.label(), "`card` at src/a.rs:1");
        assert_eq!(inner.label(), "`card` at src/a.rs:1");
    }
}
