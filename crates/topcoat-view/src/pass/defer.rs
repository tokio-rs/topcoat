use std::{future::Future, panic::Location};

use topcoat_core::context::{Cx, memoize_cache};

use crate::{
    identity::{Identity, IdentityKey, SiteKey},
    pass::scope::PassScope,
};

/// The state a render observes for one deferred load.
///
/// `Ready` hands out a borrow of the stored output: the framework owns the
/// output for the rest of the request and every pass reads it in place.
pub enum Deferred<'a, T> {
    Pending,
    Ready(&'a T),
}

/// Marks a load as allowed to arrive after the first paint.
///
/// On the first render that reaches this call, `load` is handed an owned
/// clone of the context, and the future it builds is registered with the
/// request driver and runs between passes; the call returns `Pending` and
/// the caller renders a placeholder. Once the future resolves, every later
/// pass observes `Ready` with a borrow of the output.
///
/// The call is identified by the identity of the enclosing component and
/// its own call site. A call that repeats inside a loop needs
/// [`defer_keyed`].
///
/// # Panics
///
/// Panics if the current identity is ambiguous (a repeated invocation is
/// missing its `key`) or if no pass is running on the current task.
#[track_caller]
pub fn defer<T, F, Fut>(cx: &Cx, load: F) -> Deferred<'_, T>
where
    T: Send + Sync + 'static,
    F: FnOnce(Cx) -> Fut,
    Fut: Future<Output = T> + Send + 'static,
{
    let identity = Identity::current().child(caller_site(Location::caller()));
    defer_at(cx, identity, load)
}

/// Like [`defer`], for a call that repeats: `key` tells the repetitions
/// apart.
#[track_caller]
pub fn defer_keyed<T, F, Fut>(cx: &Cx, key: impl IdentityKey, load: F) -> Deferred<'_, T>
where
    T: Send + Sync + 'static,
    F: FnOnce(Cx) -> Fut,
    Fut: Future<Output = T> + Send + 'static,
{
    let identity = Identity::current().keyed_child(caller_site(Location::caller()), key);
    defer_at(cx, identity, load)
}

fn defer_at<T, F, Fut>(cx: &Cx, identity: Identity, load: F) -> Deferred<'_, T>
where
    T: Send + Sync + 'static,
    F: FnOnce(Cx) -> Fut,
    Fut: Future<Output = T> + Send + 'static,
{
    let hash = identity.hash();
    let slot = memoize_cache(cx).defer_cache().slot::<T>(hash);
    if let Some(value) = slot.get() {
        return Deferred::Ready(value);
    }
    if slot.register() {
        let fut = load(cx.detach());
        let writer = cx.detach();
        PassScope::with(|state| {
            state.deferred.push(Box::pin(async move {
                let value = fut.await;
                memoize_cache(&writer)
                    .defer_cache()
                    .slot::<T>(hash)
                    .resolve(value);
            }));
        });
    }
    Deferred::Pending
}

fn caller_site(location: &Location<'_>) -> SiteKey {
    SiteKey::new(location.file(), location.line(), location.column(), 0)
}
