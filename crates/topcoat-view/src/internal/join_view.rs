use std::{
    pin::Pin,
    task::{Context, Poll, ready},
};

use pin_project_lite::pin_project;
use topcoat_core::{context::Cx, error::Result};

use crate::{
    PartsWriter, Swap, View,
    buffer::{ViewBuffer, ViewHandle},
};

use super::Builder;

pin_project! {
    /// A template as a [`View`]: its dynamic node positions driven
    /// concurrently, and its instruction block built from their contents.
    ///
    /// `poll_first` drives every unit toward its content; once all have
    /// resolved, the burst runs, pushing the template's block in one
    /// synchronous burst that splices the contents in position order. After
    /// that the units' updates merge into one stream of swaps.
    pub struct JoinView<U, F> {
        #[pin]
        units: U,
        // The burst; taken when the units resolve.
        burst: Option<F>,
    }
}

impl<U, F> JoinView<U, F>
where
    U: JoinUnits,
    F: FnOnce(&mut Builder<'_, '_, '_>, U::Contents),
{
    #[must_use]
    pub fn new(units: U, burst: F) -> Self {
        Self {
            units,
            burst: Some(burst),
        }
    }
}

impl<U, F> View for JoinView<U, F>
where
    U: JoinUnits + Send,
    F: FnOnce(&mut Builder<'_, '_, '_>, U::Contents) + Send,
{
    fn poll_first(
        self: Pin<&mut Self>,
        cx: &Cx,
        task: &mut Context<'_>,
        buf: &mut ViewBuffer,
    ) -> Poll<Result<ViewHandle>> {
        let mut this = self.project();
        ready!(this.units.as_mut().poll_contents(cx, task, buf))?;
        let contents = this.units.take_contents();
        let burst = this
            .burst
            .take()
            .expect("`poll_first` called again after it returned `Ready`");
        let view = PartsWriter::block(buf, |parts| burst(&mut Builder { cx, parts }, contents));
        Poll::Ready(Ok(view))
    }

    fn poll_swap(
        self: Pin<&mut Self>,
        cx: &Cx,
        task: &mut Context<'_>,
        buf: &mut ViewBuffer,
    ) -> Poll<Option<Result<Swap>>> {
        self.project().units.poll_swap(cx, task, buf)
    }
}

/// The units a [`JoinView`] drives: one level of the template's position
/// list.
///
/// The positions build different view types, so the expansion nests a
/// [`JoinUnit`] per position, terminated by `()`. The contents come back in
/// the same nested shape, destructured by the burst.
pub trait JoinUnits {
    /// The units' contents, in position order: nested
    /// `(ViewHandle, ...)` pairs terminated by `()`.
    type Contents;

    /// Polls every unit still waiting toward its content; ready when all
    /// have resolved, or with the first unit's error.
    fn poll_contents(
        self: Pin<&mut Self>,
        cx: &Cx,
        task: &mut Context<'_>,
        buf: &mut ViewBuffer,
    ) -> Poll<Result<()>>;

    /// Takes the resolved contents out of the units.
    fn take_contents(self: Pin<&mut Self>) -> Self::Contents;

    /// Polls the units for the next update; ready with `None` once every
    /// unit has no further updates.
    fn poll_swap(
        self: Pin<&mut Self>,
        cx: &Cx,
        task: &mut Context<'_>,
        buf: &mut ViewBuffer,
    ) -> Poll<Option<Result<Swap>>>;
}

impl JoinUnits for () {
    type Contents = ();

    fn poll_contents(
        self: Pin<&mut Self>,
        _cx: &Cx,
        _task: &mut Context<'_>,
        _buf: &mut ViewBuffer,
    ) -> Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn take_contents(self: Pin<&mut Self>) -> Self::Contents {}

    fn poll_swap(
        self: Pin<&mut Self>,
        _cx: &Cx,
        _task: &mut Context<'_>,
        _buf: &mut ViewBuffer,
    ) -> Poll<Option<Result<Swap>>> {
        Poll::Ready(None)
    }
}

pin_project! {
    /// One dynamic position of a [`JoinView`], linked to the rest.
    pub struct JoinUnit<V, Rest> {
        #[pin]
        view: V,
        // The unit's content, held until the burst takes it.
        content: Option<ViewHandle>,
        // Whether the view reported it has no further updates.
        done: bool,
        #[pin]
        rest: Rest,
    }
}

impl<V, Rest> JoinUnit<V, Rest>
where
    V: View,
    Rest: JoinUnits,
{
    #[must_use]
    pub fn new(view: V, rest: Rest) -> Self {
        Self {
            view,
            content: None,
            done: false,
            rest,
        }
    }
}

impl<V, Rest> JoinUnits for JoinUnit<V, Rest>
where
    V: View,
    Rest: JoinUnits,
{
    type Contents = (ViewHandle, Rest::Contents);

    fn poll_contents(
        self: Pin<&mut Self>,
        cx: &Cx,
        task: &mut Context<'_>,
        buf: &mut ViewBuffer,
    ) -> Poll<Result<()>> {
        let this = self.project();
        let mut ready = true;
        if this.content.is_none() {
            match this.view.poll_first(cx, task, buf) {
                Poll::Ready(Ok(content)) => *this.content = Some(content),
                Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                Poll::Pending => ready = false,
            }
        }
        match this.rest.poll_contents(cx, task, buf) {
            Poll::Ready(Ok(())) => {}
            Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
            Poll::Pending => ready = false,
        }
        if ready {
            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
        }
    }

    fn take_contents(self: Pin<&mut Self>) -> Self::Contents {
        let this = self.project();
        let content = this
            .content
            .take()
            .expect("every unit resolved its content");
        (content, this.rest.take_contents())
    }

    fn poll_swap(
        self: Pin<&mut Self>,
        cx: &Cx,
        task: &mut Context<'_>,
        buf: &mut ViewBuffer,
    ) -> Poll<Option<Result<Swap>>> {
        let this = self.project();
        let mut pending = false;
        if !*this.done {
            match this.view.poll_swap(cx, task, buf) {
                Poll::Ready(Some(item)) => return Poll::Ready(Some(item)),
                Poll::Ready(None) => *this.done = true,
                Poll::Pending => pending = true,
            }
        }
        match this.rest.poll_swap(cx, task, buf) {
            Poll::Ready(Some(item)) => Poll::Ready(Some(item)),
            Poll::Ready(None) if !pending => Poll::Ready(None),
            Poll::Ready(None) | Poll::Pending => Poll::Pending,
        }
    }
}
