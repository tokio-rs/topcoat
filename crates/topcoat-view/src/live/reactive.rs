use std::{
    future::{Future, poll_fn},
    pin::Pin,
    task::Poll,
};

use topcoat_core::context::Cx;

/// A value a view can consume reactively: a state to render now, and zero or
/// more replacement states later.
///
/// This is the entire interface between a reactive expression and the view
/// construct consuming it. The method is for generated code, not for
/// application code, which only creates reactive values and hands them to a
/// consuming construct.
pub trait Reactive: Send {
    /// The state the consuming construct's arms are written against.
    type State: Send;

    /// Yields the state to render now on the first call and replacement
    /// states on later calls; `None` means no further change is possible and
    /// the consuming construct retires.
    fn next_state(&mut self) -> impl Future<Output = Option<Self::State>> + Send;
}

/// The two states of a deferred load.
#[derive(Debug)]
pub enum Deferred<T> {
    /// The data has not arrived yet.
    Pending,
    /// The data arrived, delivered owned, exactly as `.await` would have.
    Ready(T),
}

/// A deferred load: a value standing for a future's output before it exists.
///
/// Created with [`defer`]. A `Defer` owns its future and is lazy: nothing
/// polls the future until a consuming construct asks for the first state, so
/// a `Defer` dropped unconsumed never runs at all.
#[must_use = "a `Defer` runs only once a view consumes it"]
pub struct Defer<F: Future> {
    state: DeferState<F>,
}

enum DeferState<F: Future> {
    /// Not yet polled; the first state gives the future its first poll.
    Fresh(Pin<Box<F>>),
    /// Polled once and pending; the next state awaits the future.
    Waiting(Pin<Box<F>>),
    /// The output was delivered; no further change is possible.
    Done,
}

/// Wraps a future instead of awaiting it, returning a [`Defer`] to render in
/// its place.
///
/// The first state is [`Deferred::Pending`] when the future needs time and
/// [`Deferred::Ready`] immediately when the data is already at hand; the
/// output is delivered owned, once. Inside a view the macro supplies the
/// `cx` argument; outside one it is passed explicitly.
#[must_use = "a `Defer` runs only once a view consumes it"]
pub fn defer<F>(_cx: &Cx, future: F) -> Defer<F>
where
    F: Future + Send,
{
    Defer {
        state: DeferState::Fresh(Box::pin(future)),
    }
}

impl<F> Reactive for Defer<F>
where
    F: Future + Send,
    F::Output: Send,
{
    type State = Deferred<F::Output>;

    async fn next_state(&mut self) -> Option<Self::State> {
        match std::mem::replace(&mut self.state, DeferState::Done) {
            DeferState::Fresh(mut future) => {
                // Laziness ends here: the future's first poll happens at
                // consumption. One poll without waiting, so data already at
                // hand reports `Ready` on the first paint.
                let first = poll_fn(|task| Poll::Ready(future.as_mut().poll(task))).await;
                match first {
                    Poll::Ready(output) => Some(Deferred::Ready(output)),
                    Poll::Pending => {
                        self.state = DeferState::Waiting(future);
                        Some(Deferred::Pending)
                    }
                }
            }
            DeferState::Waiting(future) => Some(Deferred::Ready(future.await)),
            DeferState::Done => None,
        }
    }
}
