use std::{
    any::Any,
    convert::Infallible,
    pin::Pin,
    sync::Mutex,
    task::{Context, Poll},
};

use pin_project::pin_project;

use crate::context::Cx;

pub enum MaybeAborted<T> {
    Completed(T),
    Aborted(Box<dyn Any>),
}

pub(crate) struct AbortStore {
    inner: Mutex<Option<Box<dyn Any + Send + Sync>>>,
}

impl AbortStore {
    pub(crate) fn new() -> Self {
        Self {
            inner: Mutex::new(None),
        }
    }

    fn abort(&self, value: Box<dyn Any + Send + Sync>) {
        let old = self.inner.lock().unwrap().replace(value);
        if old.is_some() {
            panic!("aborted request context that was already aborted");
        }
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

#[pin_project]
pub(crate) struct WatchAbort<'a, F> {
    cx: &'a Cx,
    #[pin]
    f: F,
}

impl<'a, F> WatchAbort<'a, F> {
    pub(crate) fn new(cx: &'a Cx, f: F) -> Self {
        Self { cx, f }
    }
}

impl<F> Future for WatchAbort<'_, F>
where
    F: Future,
{
    type Output = MaybeAborted<<F as Future>::Output>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(value) = self.cx.abort.take() {
            return Poll::Ready(MaybeAborted::Aborted(value));
        }

        let this = self.project();
        match this.f.poll(cx) {
            Poll::Ready(value) => Poll::Ready(MaybeAborted::Completed(value)),
            Poll::Pending => Poll::Pending,
        }
    }
}

pub struct Abort<'a> {
    cx: &'a Cx,
    value: Option<Box<dyn Any + Send + Sync>>,
}

impl<'a> Abort<'a> {
    pub fn new(cx: &'a Cx, value: Box<dyn Any + Send + Sync>) -> Self {
        Self {
            cx,
            value: Some(value),
        }
    }
}

impl Future for Abort<'_> {
    type Output = Infallible;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.cx.abort.abort(self.value.take().unwrap());
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

pub async fn abort(cx: &Cx, value: Box<dyn Any + Send + Sync>) -> ! {
    match Abort::new(cx, value).await {}
}
