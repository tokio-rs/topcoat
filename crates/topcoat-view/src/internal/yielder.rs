use std::{
    cell::Cell,
    mem,
    pin::Pin,
    task::{Context, Poll},
};

use pin_project_lite::pin_project;
use topcoat_core::error::Result;

use crate::{View, ViewFirst, ViewSwap};

/// Polls `body` in first-content mode and takes any content it yields.
///
/// A yielded value accompanies `Poll::Pending`. With no yielded value, the
/// body may still be waiting, or it may have completed or failed.
pub(super) fn poll_first<Fut: Future>(
    body: Pin<&mut Fut>,
    cx: &mut Context<'_>,
) -> (Poll<Fut::Output>, Option<ViewFirst>) {
    let _guard = YieldFirstGuard::new();
    let _input_guard = DriveInputGuard::new();
    DRIVE_INPUT.set(Some(DriveInput::First));
    (body.poll(cx), YIELD_FIRST.take())
}

/// Polls `body` in swap mode and takes any replacement it yields.
///
/// A yielded swap accompanies `Poll::Pending`. Completion of a driven view
/// lets the body continue, so the body may yield another view's content or
/// wait for other work before it finishes.
pub(super) fn poll_swap<Fut: Future>(
    body: Pin<&mut Fut>,
    cx: &mut Context<'_>,
) -> (Poll<Fut::Output>, Option<ViewSwap>) {
    let _guard = YieldSwapGuard::new();
    let _input_guard = DriveInputGuard::new();
    DRIVE_INPUT.set(Some(DriveInput::Swap));
    (body.poll(cx), YIELD_SWAP.take())
}

thread_local! {
    /// The view method requested by the innermost polling helper.
    static DRIVE_INPUT: Cell<Option<DriveInput>> = const { Cell::new(None) };
    /// First content waiting for the current first-content poll to return.
    static YIELD_FIRST: Cell<Option<ViewFirst>> = const { Cell::new(None) };
    /// A replacement waiting for the current swap poll to return.
    static YIELD_SWAP: Cell<Option<ViewSwap>> = const { Cell::new(None) };
}

/// The view method to call while driving the current body.
#[derive(Debug, Clone, Copy)]
enum DriveInput {
    First,
    Swap,
}

/// Defines a guard that clears a slot and restores its enclosing value on drop.
macro_rules! guard {
    ($guard:ident, $tl:ident: $ty:ty) => {
        struct $guard {
            prev: Option<$ty>,
        }

        impl $guard {
            fn new() -> Self {
                Self { prev: $tl.take() }
            }
        }

        impl Drop for $guard {
            fn drop(&mut self) {
                $tl.set(mem::take(&mut self.prev));
            }
        }
    };
}

guard!(YieldFirstGuard, YIELD_FIRST: ViewFirst);
guard!(YieldSwapGuard, YIELD_SWAP: ViewSwap);
guard!(DriveInputGuard, DRIVE_INPUT: DriveInput);

pin_project! {
    /// A future that drives a view and yields its content to the enclosing poll.
    ///
    /// Poll it only through this module's helpers. Their mode selects the
    /// view method directly; the drive does not track whether the view has
    /// emitted first content or reported itself as no longer live.
    ///
    /// Yielding content leaves the future pending. It resolves when the
    /// view reports no more swaps, or returns an error from either method.
    /// If another drive has filled the slot, this drive leaves its view
    /// unpolled and wakes the task so it can try again after the slot is read.
    pub(super) struct DriveFuture<V> {
        #[pin]
        view: V,
    }
}

impl<V: View> DriveFuture<V> {
    /// Wraps a view to drive when the returned future is awaited.
    pub(super) fn new(view: V) -> Self {
        Self { view }
    }
}

impl<V: View> Future for DriveFuture<V> {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let Some(input) = DRIVE_INPUT.get() else {
            panic!("polled DriveFuture without DriveInput");
        };

        match input {
            DriveInput::First => {
                if YIELD_FIRST.with(|cell| {
                    let value = cell.take();
                    let occupied = value.is_some();
                    cell.set(value);
                    occupied
                }) {
                    cx.waker().wake_by_ref();
                    return Poll::Pending;
                }
                match this.view.poll_first(cx) {
                    Poll::Ready(Ok(first)) => YIELD_FIRST.with(|cell| cell.set(Some(first))),
                    Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                    Poll::Pending => return Poll::Pending,
                }
            }
            DriveInput::Swap => {
                if YIELD_SWAP.with(|cell| {
                    let value = cell.take();
                    let occupied = value.is_some();
                    cell.set(value);
                    occupied
                }) {
                    cx.waker().wake_by_ref();
                    return Poll::Pending;
                }
                match this.view.poll_swap(cx) {
                    Poll::Ready(Ok(Some(swap))) => YIELD_SWAP.with(|cell| cell.set(Some(swap))),
                    Poll::Ready(Ok(None)) => return Poll::Ready(Ok(())),
                    Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                    Poll::Pending => return Poll::Pending,
                }
            }
        }

        Poll::Pending
    }
}
