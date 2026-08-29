use std::{
    pin::Pin,
    task::{Context, Poll, ready},
};

use pin_project_lite::pin_project;
use topcoat_core::{context::Cx, error::Result};

use super::Builder;
use crate::{
    Step, View,
    buffer::{ViewBufferScope, ViewHandle},
};

pin_project! {
    /// A template as a [`View`]: its dynamic node positions driven
    /// concurrently, and its instruction block built from their contents.
    ///
    /// Every unit is driven toward its content; once all have resolved, the
    /// burst runs, pushing the template's block into the buffer of the
    /// build in one synchronous burst that splices the contents in position
    /// order. After that the units' updates merge into one stream of swaps.
    pub struct JoinView<'a, U, F> {
        cx: &'a Cx,
        #[pin]
        units: U,
        // The burst; taken when the units resolve.
        burst: Option<F>,
    }
}

impl<'a, U, F> JoinView<'a, U, F>
where
    U: JoinUnits,
    F: FnOnce(&mut Builder<'_, '_, '_>, U::Contents),
{
    #[must_use]
    pub fn new(cx: &'a Cx, units: U, burst: F) -> Self {
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
    fn poll_first(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Step>> {
        let mut this = self.project();
        if this.burst.is_none() {
            return this.units.poll_swap(cx);
        }
        ready!(this.units.as_mut().poll_contents(cx))?;
        let contents = this.units.as_mut().take_contents();
        let burst = this.burst.take().expect("the burst is still to run");
        let content =
            ViewBufferScope::block(|parts| burst(&mut Builder::new(this.cx, parts), contents));
        Poll::Ready(Ok(Step::Content {
            content,
            live: this.units.is_live(),
        }))
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

    /// Polls the units for the next update: a [`Step::Swap`], or
    /// [`Step::Done`] once every unit has no further updates.
    fn poll_swap(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Step>>;
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

    fn poll_swap(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<Step>> {
        Poll::Ready(Ok(Step::Done))
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
                Poll::Ready(Ok(Step::Content { content, live })) => {
                    *this.content = Some(content);
                    *this.done = !live;
                }
                Poll::Ready(Ok(Step::Swap { .. } | Step::Done)) => {
                    panic!("a view swapped or completed before its first content")
                }
                Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                Poll::Pending => ready = false,
            }
        }
        match this.rest.poll_contents(cx) {
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

    fn is_live(&self) -> bool {
        !self.done || self.rest.is_live()
    }

    fn poll_swap(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Step>> {
        let this = self.project();
        let mut pending = false;
        if !*this.done {
            match this.view.poll_first(cx) {
                Poll::Ready(Ok(Step::Swap { swap, live })) => {
                    *this.done = !live;
                    return Poll::Ready(Ok(Step::Swap {
                        swap,
                        live: live || this.rest.is_live(),
                    }));
                }
                Poll::Ready(Ok(Step::Done)) => *this.done = true,
                Poll::Ready(Ok(Step::Content { .. })) => panic!("a view produced content twice"),
                Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                Poll::Pending => pending = true,
            }
        }
        match this.rest.poll_swap(cx) {
            Poll::Ready(Ok(Step::Swap { swap, live })) => Poll::Ready(Ok(Step::Swap {
                swap,
                live: live || !*this.done,
            })),
            Poll::Ready(Ok(Step::Done)) if !pending => Poll::Ready(Ok(Step::Done)),
            Poll::Ready(Ok(Step::Done)) | Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(Step::Content { .. })) => panic!("a view produced content twice"),
            Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
        }
    }
}
