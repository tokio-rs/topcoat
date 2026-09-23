//! Stopping a future early with a value.
//!
//! Sometimes code deep inside a future needs to stop the whole future and hand
//! a value back to its caller, without passing that value up through the
//! `Output` type of every future in between. This module does that:
//!
//! - [`WatchAbort`] wraps a future and watches an [`AbortStore`].
//! - Code running inside that future calls [`abort`] to put a value into the store and stop.
//! - [`WatchAbort`] then resolves to [`MaybeAborted::Aborted`] with that value and drops the rest
//!   of the wrapped future. If nothing aborts, it resolves to [`MaybeAborted::Completed`] with the
//!   future's output.
//!
//! The value is passed as a `Box<dyn Any>`, so the caller gets the concrete
//! type back with [`downcast`](Box::downcast).
//!
//! ```rust
//! # use std::boxed::Box;
//! # use topcoat_core::abort::{AbortStore, WatchAbort, MaybeAborted, abort};
//! # async fn example() {
//! let store = AbortStore::new();
//! let outcome = WatchAbort::new(&store, async {
//!     abort(&store, Box::new(42i32)).await;
//!     unreachable!("the future stops at the abort point");
//! })
//! .await;
//!
//! match outcome {
//!     MaybeAborted::Completed(value) => { /* the future finished normally */ }
//!     MaybeAborted::Aborted(value) => {
//!         assert_eq!(*value.downcast::<i32>().unwrap(), 42);
//!     }
//! }
//! # }
//! ```

use std::{
    any::Any,
    convert::Infallible,
    pin::Pin,
    sync::Mutex,
    task::{Context, Poll},
};

use pin_project_lite::pin_project;

/// The outcome of a [`WatchAbort`] future.
///
/// Either the wrapped future ran to completion, or it was stopped by
/// [`abort`] before finishing.
pub enum MaybeAborted<T> {
    /// The wrapped future finished normally, producing this output.
    Completed(T),
    /// The wrapped future was aborted with this value, as passed to
    /// [`abort`]. Get the original type back with
    /// [`downcast`](Box::downcast).
    Aborted(Box<dyn Any>),
}

/// A slot shared by a [`WatchAbort`] and the [`abort`] calls running inside
/// it.
///
/// It holds the value passed to [`abort`] until the [`WatchAbort`] takes it
/// out. Aborting the same store a second time before the first value was
/// taken out panics.
#[derive(Default)]
pub struct AbortStore {
    inner: Mutex<Option<Box<dyn Any + Send + Sync>>>,
}

impl AbortStore {
    /// Creates an empty store.
    #[must_use]
    pub fn new() -> Self {
        AbortStore::default()
    }

    fn abort(&self, value: Box<dyn Any + Send + Sync>) {
        let old = self.inner.lock().unwrap().replace(value);
        assert!(
            old.is_none(),
            "aborted request context that was already aborted"
        );
    }

    fn take(&self) -> Option<Box<dyn Any + Send + Sync>> {
        self.inner.lock().unwrap().take()
    }
}

impl std::fmt::Debug for AbortStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AbortStore").finish()
    }
}

pin_project! {
    /// A future that runs `f` while watching `store`.
    ///
    /// If code inside `f` calls [`abort`] on the same store, this future
    /// resolves to [`MaybeAborted::Aborted`] with the value and `f` is
    /// dropped. Otherwise it resolves to [`MaybeAborted::Completed`] with
    /// `f`'s output.
    pub struct WatchAbort<'a, F> {
        store: &'a AbortStore,
        #[pin]
        f: F,
    }
}

impl<'a, F> WatchAbort<'a, F> {
    /// Wraps `f` so that an [`abort`] on `store` stops it.
    pub fn new(store: &'a AbortStore, f: F) -> Self {
        Self { store, f }
    }
}

impl<F> Future for WatchAbort<'_, F>
where
    F: Future,
{
    type Output = MaybeAborted<<F as Future>::Output>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(value) = self.store.take() {
            return Poll::Ready(MaybeAborted::Aborted(value));
        }

        let this = self.project();
        match this.f.poll(cx) {
            Poll::Ready(value) => Poll::Ready(MaybeAborted::Completed(value)),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// A future that puts a value into an [`AbortStore`] and then never
/// completes.
///
/// On its first poll it stores the value and wakes itself, so the surrounding
/// [`WatchAbort`] is polled again, sees the value, and resolves. Most code
/// calls [`abort`] instead of using this type directly.
pub struct Abort<'a> {
    store: &'a AbortStore,
    value: Option<Box<dyn Any + Send + Sync>>,
}

impl<'a> Abort<'a> {
    /// Creates a future that aborts `store` with `value` when polled.
    pub fn new(store: &'a AbortStore, value: Box<dyn Any + Send + Sync>) -> Self {
        Self {
            store,
            value: Some(value),
        }
    }
}

impl Future for Abort<'_> {
    type Output = Infallible;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.store.abort(self.value.take().unwrap());
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

/// Aborts the surrounding [`WatchAbort`] with `value`.
///
/// Puts `value` into `store` and yields, so the [`WatchAbort`] watching
/// `store` resolves to [`MaybeAborted::Aborted`]. This call never returns: the
/// future it runs in stops here and is dropped.
///
/// # Panics
///
/// Panics if `store` already holds a value from an earlier abort that was not
/// taken out yet.
pub async fn abort(store: &AbortStore, value: Box<dyn Any + Send + Sync>) -> ! {
    match Abort::new(store, value).await {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn completes_with_inner_output() {
        let store = AbortStore::new();
        let outcome = WatchAbort::new(&store, async { 7u32 }).await;
        match outcome {
            MaybeAborted::Completed(value) => assert_eq!(value, 7),
            MaybeAborted::Aborted(_) => panic!("expected completion"),
        }
    }

    #[tokio::test]
    async fn aborts_with_value() {
        let store = AbortStore::new();
        let outcome = WatchAbort::new(&store, async {
            abort(&store, Box::new(42i32)).await;
        })
        .await;
        match outcome {
            MaybeAborted::Aborted(value) => assert_eq!(*value.downcast::<i32>().unwrap(), 42),
            MaybeAborted::Completed(()) => panic!("expected abort"),
        }
    }

    #[tokio::test]
    #[allow(unreachable_code, reason = "`abort` never returns, by design")]
    async fn abort_skips_remaining_work() {
        let store = AbortStore::new();
        let outcome = WatchAbort::new(&store, async {
            abort(&store, Box::new("stop".to_string())).await;
            unreachable!("the future continued past the abort point");
        })
        .await;
        match outcome {
            MaybeAborted::Aborted(value) => {
                assert_eq!(*value.downcast::<String>().unwrap(), "stop");
            }
            MaybeAborted::Completed(()) => panic!("expected abort"),
        }
    }

    #[tokio::test]
    #[should_panic(expected = "already aborted")]
    async fn double_abort_panics() {
        let store = AbortStore::new();
        store.abort(Box::new(1i32));
        store.abort(Box::new(2i32));
    }
}
