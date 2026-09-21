use std::{
    cell::Cell,
    mem,
    pin::Pin,
    task::{Context, Poll},
};

use pin_project_lite::pin_project;
use topcoat_core::error::Result;

use crate::{View, ViewFirst, ViewPass, ViewSwap};

pub(super) fn poll_first<Fut: Future>(
    body: Pin<&mut Fut>,
    cx: &mut Context<'_>,
) -> (Poll<Fut::Output>, Option<ViewFirst>) {
    let _guard = YieldFirstGuard::new();
    DRIVE_INPUT.set(Some(DriveInput::First));
    (body.poll(cx), YIELD_FIRST.take())
}

pub(super) fn poll_swap<Fut: Future>(
    body: Pin<&mut Fut>,
    cx: &mut Context<'_>,
    pass: ViewPass,
) -> (Poll<Fut::Output>, Option<ViewSwap>) {
    let _guard = YieldSwapGuard::new();
    DRIVE_INPUT.set(Some(DriveInput::Swap { pass }));
    (body.poll(cx), YIELD_SWAP.take())
}

thread_local! {
    static DRIVE_INPUT: Cell<Option<DriveInput>> = const { Cell::new(None) };
    static YIELD_FIRST: Cell<Option<ViewFirst>> = const { Cell::new(None) };
    static YIELD_SWAP: Cell<Option<ViewSwap>> = const { Cell::new(None) };
}

#[derive(Debug)]
enum DriveInput {
    First,
    Swap { pass: ViewPass },
}

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

pin_project! {
    pub(super) struct DriveFuture<V> {
        #[pin]
        view: V,
    }
}

impl<V: View> DriveFuture<V> {
    pub(super) fn new(view: V) -> Self {
        Self { view }
    }
}

impl<V: View> Future for DriveFuture<V> {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let Some(input) = DRIVE_INPUT.take() else {
            panic!("polled DriveFuture without YieldInput");
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
            DriveInput::Swap { pass } => {
                if YIELD_SWAP.with(|cell| {
                    let value = cell.take();
                    let occupied = value.is_some();
                    cell.set(value);
                    occupied
                }) {
                    cx.waker().wake_by_ref();
                    return Poll::Pending;
                }
                match this.view.poll_swap(cx, pass) {
                    Poll::Ready(Ok(Some(swap))) => YIELD_SWAP.with(|cell| cell.set(Some(swap))),
                    Poll::Ready(Ok(None)) => return Poll::Ready(Ok(())),
                    Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                    Poll::Pending => return Poll::Pending,
                }
            }
        };

        Poll::Pending
    }
}
