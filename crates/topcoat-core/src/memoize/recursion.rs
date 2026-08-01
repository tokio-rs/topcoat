use std::{
    ptr,
    sync::atomic::{AtomicPtr, Ordering},
};

// Its address identifies the synchronous call stack or async poll currently running on a thread.
thread_local!(static TOKEN: u8 = const { 0 });

/// Detects reentry into an initializer from the call stack or poll already running it.
///
/// A memoized value is produced once, so a nested call for the same key would wait on a result
/// only the waiting caller can produce. Reporting that as a panic keeps it from surfacing as a
/// hang. Callers reaching the same key from another thread or task are ordinary concurrency and
/// are left to wait.
#[derive(Default)]
pub(super) struct Guard {
    // This is recursion metadata, not the initialization lock. `OnceLock::get_or_init` and
    // `OnceCell::get_or_init` select one initializer at a time, so only the active initializer
    // writes `owner`.
    //
    // Relaxed ordering is enough because this pointer publishes no data. A recursive read happens
    // on the initializing thread after its own store. A caller on another thread may see null or a
    // foreign token, but either result safely falls through to the once cell for synchronization.
    owner: AtomicPtr<u8>,
}

impl Guard {
    /// Panics if the caller is already inside this guard's [`scope`](Self::scope), naming the
    /// memoized function `F`.
    #[inline]
    pub(super) fn assert_not_recursive<F>(&self) {
        let owner = self.owner.load(Ordering::Relaxed);
        if !owner.is_null() && TOKEN.with(|token| owner == ptr::from_ref(token).cast_mut()) {
            panic!(
                "recursive `#[memoize]` initialization of `{}` with the same arguments",
                std::any::type_name::<F>()
            );
        }
    }

    /// Runs `f` with this guard marked as owned by the current thread.
    ///
    /// Ownership lasts only for the call, so an async initializer wraps each individual poll
    /// rather than the future as a whole.
    #[inline]
    pub(super) fn scope<R>(&self, f: impl FnOnce() -> R) -> R {
        TOKEN.with(|token| {
            debug_assert!(self.owner.load(Ordering::Relaxed).is_null());
            self.owner
                .store(ptr::from_ref(token).cast_mut(), Ordering::Relaxed);
            let _scope = Scope { owner: &self.owner };
            f()
        })
    }
}

/// Releases ownership of a [`Guard`] when the scope that took it ends, including through an
/// unwind, so a panicking initializer can be retried.
struct Scope<'a> {
    owner: &'a AtomicPtr<u8>,
}

impl Drop for Scope<'_> {
    #[inline]
    fn drop(&mut self) {
        self.owner.store(ptr::null_mut(), Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Barrier;

    use super::*;

    #[test]
    fn scope_is_released_after_it_ends() {
        let guard = Guard::default();

        guard.scope(|| {});

        guard.assert_not_recursive::<fn()>();
    }

    #[test]
    #[should_panic(expected = "recursive `#[memoize]` initialization")]
    fn reentry_from_the_owning_stack_panics() {
        let guard = Guard::default();

        guard.scope(|| guard.assert_not_recursive::<fn()>());
    }

    #[test]
    fn scope_on_another_thread_is_not_recursive() {
        let guard = Guard::default();
        let entered = Barrier::new(2);
        let release = Barrier::new(2);

        std::thread::scope(|scope| {
            scope.spawn(|| {
                guard.scope(|| {
                    entered.wait();
                    release.wait();
                });
            });
            entered.wait();

            guard.assert_not_recursive::<fn()>();

            release.wait();
        });
    }
}
