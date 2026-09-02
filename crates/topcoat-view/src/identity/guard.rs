use std::marker::PhantomData;

use super::{CURRENT, Identity, IdentityKey, SiteKey};

/// Installs an identity for exactly the duration of a synchronous region.
///
/// Creating a guard derives the identity one level down from the installed
/// one and swaps it in; dropping the guard swaps the previous identity
/// back, also when the region panics. The guard is the synchronous
/// counterpart of [`IdentityView`](super::IdentityView), for regions that
/// build in a single burst.
#[must_use = "the identity is uninstalled when the guard drops"]
pub struct IdentityGuard {
    identity: Identity,
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
    pub fn enter_keyed(site: SiteKey, key: impl IdentityKey) -> Self {
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
            identity,
            prev: CURRENT.replace(Some(identity)),
            _not_send: PhantomData,
        }
    }

    /// Returns the identity this guard installed.
    ///
    /// Hands the derived identity back so a region that builds inside the
    /// guard and resolves outside it, such as a component invocation whose
    /// props are built here and whose view is polled later, can install
    /// the same identity again.
    #[must_use]
    pub fn identity(&self) -> Identity {
        self.identity
    }
}

impl Drop for IdentityGuard {
    fn drop(&mut self) {
        CURRENT.set(self.prev);
    }
}

#[cfg(test)]
mod tests {
    use std::panic::catch_unwind;

    use super::*;

    const SITE_A: SiteKey = SiteKey::new(file!(), line!(), column!(), 0);
    const SITE_B: SiteKey = SiteKey::new(file!(), line!(), column!(), 0);

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
}
