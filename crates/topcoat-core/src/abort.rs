//! Aborting a future early with a value.
//!
//! Wrap a future in [`WatchAbort`] and give it an [`AbortStore`]. Code inside
//! the future can call [`abort`] with the same store to stop the work and
//! return a value through [`MaybeAborted::Aborted`]. The wrapped future is
//! dropped. If the work finishes normally, the result is
//! [`MaybeAborted::Completed`].
//!
//! The value travels as a type-erased `Box<dyn Any>`, so the watcher recovers
//! the concrete type with [`downcast`](Box::downcast).
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
/// Either the wrapped future ran to completion, or it was aborted via [`abort`]
/// before finishing.
pub enum MaybeAborted<T> {
    /// The wrapped future finished normally, producing this output.
    Completed(T),
    /// The wrapped future was aborted, carrying the type-erased value passed to
    /// [`abort`]. Recover the original type with
    /// [`downcast`](Box::downcast).
    Aborted(Box<dyn Any>),
}

/// A one-shot slot shared between a [`WatchAbort`] and the [`abort`] calls
/// running inside it.
///
/// Holds an abort value until [`WatchAbort`] takes it. A second abort before
/// the value is taken panics.
#[derive(Default)]
pub struct AbortStore {
    inner: Mutex<Option<Box<dyn Any + Send + Sync>>>,
}

impl AbortStore {
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
    /// A future that drives `f` to completion while watching `store`.
    ///
    /// If anything inside `f` calls [`abort`] on the same store, this future
    /// resolves to [`MaybeAborted::Aborted`] with the stored value and `f` is
    /// dropped. Otherwise it resolves to [`MaybeAborted::Completed`] with `f`'s
    /// output.
    pub struct WatchAbort<'a, F> {
        store: &'a AbortStore,
        #[pin]
        f: F,
    }
}

impl<'a, F> WatchAbort<'a, F> {
    /// Wrap `f` so that aborts on `store` short-circuit it.
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

/// A future that stores `value` into the [`AbortStore`] and then never
/// completes.
///
/// The first poll stores the value and yields. [`WatchAbort`] then observes
/// the value and stops the wrapped future.
pub struct Abort<'a> {
    store: &'a AbortStore,
    value: Option<Box<dyn Any + Send + Sync>>,
}

impl<'a> Abort<'a> {
    /// Create a future that will abort `store` with `value`.
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

/// Abort the surrounding [`WatchAbort`] with `value`.
///
/// Stores `value` and yields to [`WatchAbort`], which returns
/// [`MaybeAborted::Aborted`]. This call never returns. The surrounding future
/// stops here and is dropped by the watcher.
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
