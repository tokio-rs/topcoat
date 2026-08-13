//! Support for tests that drive a live render by hand: a driver that owns
//! the render scope and polls chunk by chunk, and a multi-fire reactive
//! channel fed from the test.
//!
//! For this crate's tests only; nothing here is part of the crate's API.

use std::{
    collections::VecDeque,
    future::{Future, poll_fn},
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
};

use topcoat_core::{context::Cx, error::Result};

use crate::{
    Formatter, View,
    buffer::{CellId, Renderer, ViewBuffer, ViewBufferScope},
    live::{Fill, Reactive},
    view::ViewRepr,
};

/// A manual driver for one live render: owns the render scope, installs it
/// around every poll, and exposes the rendered document between polls.
///
/// The driver polls with a no-op waker and sweeps until quiescent, so
/// futures inside the render must complete by being re-polled after the test
/// changes some state, never by waking the task.
pub struct TestRender<F> {
    future: Pin<Box<F>>,
    /// The render scope between polls; installed for the duration of each.
    buffer: Option<Box<ViewBuffer>>,
    /// The root cell the render's fill delivers the document to.
    root: CellId,
}

impl<F> TestRender<F>
where
    F: Future<Output = Result<()>> + Send,
{
    /// Creates the render: a fresh scope, a root cell, and the root future
    /// minted from the fill that delivers to it.
    pub fn new(render: impl FnOnce(Fill) -> F) -> Self {
        let mut buffer = Box::new(ViewBuffer::new());
        let root = buffer.new_cell(true);
        Self {
            future: Box::pin(render(Fill::new(root))),
            buffer: Some(buffer),
            root,
        }
    }

    /// Polls the render once with the scope installed, one sweep pass.
    pub fn poll(&mut self) -> Poll<Result<()>> {
        let Self { future, buffer, .. } = self;
        buffer
            .as_deref_mut()
            .expect("the driver owns the buffer between polls")
            .clear_progress();
        let mut task = Context::from_waker(Waker::noop());
        ViewBufferScope::install(buffer, || future.as_mut().poll(&mut task))
    }

    /// Polls the render until a pass makes no progress or it completes.
    ///
    /// # Panics
    ///
    /// Panics if the render never settles.
    pub fn settle(&mut self) -> Poll<Result<()>> {
        for _ in 0..10_000 {
            let poll = self.poll();
            if poll.is_ready() {
                return poll;
            }
            if !self.buffer().progress() {
                return Poll::Pending;
            }
        }
        panic!("the render never settled");
    }

    /// Takes the dirty flag: whether the document changed since the last
    /// take, meaning a driver would send a chunk.
    pub fn take_dirty(&mut self) -> bool {
        self.buffer
            .as_deref_mut()
            .expect("the driver owns the buffer between polls")
            .take_dirty()
    }

    /// Renders the document as delivered so far, `None` before the first
    /// paint.
    #[must_use]
    pub fn html(&self, cx: &Cx) -> Option<String> {
        let buffer = self.buffer();
        let view = buffer.delivered_view(self.root)?;
        Some(render_view(buffer, cx, view))
    }

    fn buffer(&self) -> &ViewBuffer {
        self.buffer
            .as_deref()
            .expect("the driver owns the buffer between polls")
    }
}

/// Executes a view against the driver's buffer, outside any scope.
fn render_view(buffer: &ViewBuffer, cx: &Cx, view: View) -> String {
    match view.repr() {
        ViewRepr::Static(body) => body.to_owned(),
        ViewRepr::Scoped {
            entry, size_hint, ..
        } => {
            let mut html = String::with_capacity(size_hint);
            let mut f = Formatter::new(&mut html);
            Renderer::new(buffer, entry).execute(cx, &mut f);
            html
        }
        ViewRepr::Owned {
            buffer,
            entry,
            size_hint,
        } => {
            let mut html = String::with_capacity(size_hint);
            let mut f = Formatter::new(&mut html);
            Renderer::new(&buffer, entry).execute(cx, &mut f);
            html
        }
    }
}

/// What the emitter and the channel share.
struct Shared<T> {
    queue: VecDeque<T>,
    closed: bool,
}

/// The sending end of a test channel: queues states for the reactive end.
pub struct Emitter<T> {
    shared: Arc<Mutex<Shared<T>>>,
}

/// A multi-fire reactive expression fed from the test.
///
/// Yields the queued states in order, is pending while the queue is empty,
/// and retires once closed and drained. There is no waker: the driver
/// re-polls the render after the test sends.
pub struct Channel<T> {
    shared: Arc<Mutex<Shared<T>>>,
}

/// Creates a test channel: an emitter for the test and a [`Reactive`]
/// channel for the render to consume.
#[must_use]
pub fn channel<T: Send>() -> (Emitter<T>, Channel<T>) {
    let shared = Arc::new(Mutex::new(Shared {
        queue: VecDeque::new(),
        closed: false,
    }));
    (
        Emitter {
            shared: shared.clone(),
        },
        Channel { shared },
    )
}

impl<T> Emitter<T> {
    /// Queues a state for the channel to yield.
    ///
    /// # Panics
    ///
    /// Panics if the channel's lock was poisoned.
    pub fn send(&self, state: T) {
        self.shared
            .lock()
            .expect("the channel lock was poisoned")
            .queue
            .push_back(state);
    }

    /// Retires the channel once its queue drains.
    ///
    /// # Panics
    ///
    /// Panics if the channel's lock was poisoned.
    pub fn close(&self) {
        self.shared
            .lock()
            .expect("the channel lock was poisoned")
            .closed = true;
    }
}

impl<T: Send> Reactive for Channel<T> {
    type State = T;

    fn next_state(&mut self) -> impl Future<Output = Option<T>> + Send {
        let shared = self.shared.clone();
        poll_fn(move |_task| {
            let mut shared = shared.lock().expect("the channel lock was poisoned");
            match shared.queue.pop_front() {
                Some(state) => Poll::Ready(Some(state)),
                None if shared.closed => Poll::Ready(None),
                None => Poll::Pending,
            }
        })
    }
}
