use std::{
    pin::Pin,
    task::{Context, Poll, ready},
};

use pin_project_lite::pin_project;
use topcoat_core::{context::Cx, error::Result};

use super::Builder;
use crate::{View, ViewFirst, ViewHandle, ViewSwap};

pin_project! {
    /// A template as a [`View`]: its dynamic node positions driven
    /// concurrently, and its instruction block built from their contents.
    ///
    /// Every unit is driven toward its content; once all have resolved, the
    /// burst runs, pushing the template's block into the buffer of the
    /// build in one synchronous burst that splices the contents in position
    /// order. After that the units' updates merge into one stream of swaps.
    pub struct JoinView<'cx, U, F> {
        cx: &'cx Cx,
        #[pin]
        units: U,
        // The burst; taken when the units resolve.
        burst: Option<F>,
    }
}

impl<'cx, U, F> JoinView<'cx, U, F>
where
    U: JoinUnits,
    F: FnOnce(&mut Builder<'_, '_, '_>, U::Contents),
{
    #[must_use]
    pub fn new(cx: &'cx Cx, units: U, burst: F) -> Self {
        Self {
            cx,
            units,
            burst: Some(burst),
        }
    }
}

impl<U, F> View for JoinView<'_, U, F>
where
    U: JoinUnits + Send,
    F: FnOnce(&mut Builder<'_, '_, '_>, U::Contents) + Send,
{
    fn poll_first(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<ViewFirst>> {
        let mut this = self.project();
        ready!(this.units.as_mut().poll_contents(cx))?;

        let contents = this.units.as_mut().take_contents();
        let burst = this.burst.take().expect("the burst runs once");
        let content = Builder::block(this.cx, |builder| burst(builder, contents));

        Poll::Ready(Ok(ViewFirst {
            content,
            live: this.units.is_live(),
        }))
    }

    fn poll_swap(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Option<ViewSwap>>> {
        self.project().units.poll_swap(cx)
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
    fn poll_contents(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>>;

    /// Takes the resolved contents out of the units.
    fn take_contents(self: Pin<&mut Self>) -> Self::Contents;

    /// Whether any unit may still update.
    fn is_live(&self) -> bool;

    /// Polls the units for the next swap, or for `None` once every unit has
    /// no further updates.
    fn poll_swap(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Option<ViewSwap>>>;
}

impl JoinUnits for () {
    type Contents = ();

    fn poll_contents(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn take_contents(self: Pin<&mut Self>) -> Self::Contents {}

    fn is_live(&self) -> bool {
        false
    }

    fn poll_swap(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<Option<ViewSwap>>> {
        Poll::Ready(Ok(None))
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

    fn poll_contents(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        let this = self.project();
        let mut ready = true;

        if this.content.is_none() {
            match this.view.poll_first(cx) {
                Poll::Ready(Ok(ViewFirst { content, live })) => {
                    *this.content = Some(content);
                    *this.done = !live;
                }
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => ready = false,
            }
        }

        match this.rest.poll_contents(cx) {
            Poll::Ready(Ok(())) => {}
            Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
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

    fn is_live(&self) -> bool {
        !self.done || self.rest.is_live()
    }

    fn poll_swap(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Option<ViewSwap>>> {
        let this = self.project();
        let mut pending = false;

        if !*this.done {
            match this.view.poll_swap(cx) {
                Poll::Ready(Ok(Some(swap))) => return Poll::Ready(Ok(Some(swap))),
                Poll::Ready(Ok(None)) => *this.done = true,
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => pending = true,
            }
        }

        match this.rest.poll_swap(cx) {
            Poll::Ready(Ok(Some(swap))) => Poll::Ready(Ok(Some(swap))),
            // The rest is done, but this unit still owes a swap.
            Poll::Ready(Ok(None)) if pending => Poll::Pending,
            Poll::Ready(Ok(None)) => Poll::Ready(Ok(None)),
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        }
    }
}
