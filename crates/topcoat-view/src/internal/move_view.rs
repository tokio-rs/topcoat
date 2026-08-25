use std::{
    cell::Cell,
    future::Ready,
    mem,
    pin::Pin,
    task::{Context, Poll, ready},
};

use pin_project_lite::pin_project;
use topcoat_core::{context::Cx, error::Result};

use crate::{
    Swap, View,
    buffer::{ViewBuffer, ViewHandle},
};

thread_local! {
    /// The exchange of the [`MoveView`] poll running on the current task, if
    /// any: installed around the poll of its body, taken by the drive the
    /// body awaits for the duration of the tree's poll.
    static EXCHANGE: Cell<Option<Exchange>> = const { Cell::new(None) };
}

/// What a [`MoveView`] and the drive inside its body pass across the body's
/// future, which has no poll parameters of its own.
struct Exchange {
    /// The buffer the enclosing poll was handed, moved in for the tree to
    /// append to and moved back out afterwards.
    buffer: ViewBuffer,
    /// What the drive produced during the poll, if anything.
    response: Option<Response>,
}

/// One result of the tree's poll, reported back to the [`MoveView`].
enum Response {
    /// The tree's first content, produced during `poll_first`.
    First(ViewHandle),
    /// One of the tree's swaps, produced during `poll_swap`.
    Swap(Swap),
}

/// The exchange installed for one poll of a [`MoveView`] body.
///
/// Creating it moves the poll's buffer into a fresh exchange in the task's
/// slot, parking whatever an enclosing poll had installed; finishing or
/// dropping it moves the buffer back and restores the parked exchange, also
/// when the poll panics.
struct Installed<'a> {
    buf: &'a mut ViewBuffer,
    previous: Option<Exchange>,
    installed: bool,
}

impl<'a> Installed<'a> {
    fn new(buf: &'a mut ViewBuffer) -> Self {
        let exchange = Exchange {
            buffer: mem::replace(buf, ViewBuffer::new()),
            response: None,
        };
        let previous = EXCHANGE.replace(Some(exchange));
        Self {
            buf,
            previous,
            installed: true,
        }
    }

    /// Uninstalls the exchange and returns the response the body left in it.
    fn finish(mut self) -> Option<Response> {
        self.uninstall()
    }

    fn uninstall(&mut self) -> Option<Response> {
        if !mem::take(&mut self.installed) {
            return None;
        }
        let exchange = EXCHANGE
            .replace(self.previous.take())
            .expect("the body's drive put the exchange back");
        *self.buf = exchange.buffer;
        exchange.response
    }
}

impl Drop for Installed<'_> {
    fn drop(&mut self) {
        self.uninstall();
    }
}

/// The exchange a drive holds for the duration of the tree's poll.
///
/// The slot is empty while the tree polls, so a `MoveView` nested in the
/// tree installs its own exchange. Dropping puts the exchange back, also
/// when the tree's poll panics.
struct Taken {
    exchange: Option<Exchange>,
}

impl Taken {
    fn new() -> Self {
        Self {
            exchange: EXCHANGE.take(),
        }
    }

    fn exchange(&mut self) -> &mut Exchange {
        self.exchange.as_mut().unwrap_or_else(|| {
            panic!("`MoveView::drive` must be awaited by the body of the `MoveView` being polled")
        })
    }
}

impl Drop for Taken {
    fn drop(&mut self) {
        EXCHANGE.set(self.exchange.take());
    }
}

pin_project! {
    /// A view owning the data of the scope it was built in.
    ///
    /// The `view!` macro wraps a template's body in this type. The body is
    /// an `async move` block: it captures every value the template uses, so
    /// the template has no lifetime tied to the scope it was written in.
    /// Inside the block, the body builds the template's view borrowing those
    /// captures and awaits [`drive`](Self::drive), which polls it in place
    /// against the buffer this view is polled with. The built view never
    /// leaves the block, so its borrows stay valid for as long as the body
    /// lives.
    ///
    /// The body type defaults so `<MoveView>::drive` names the function
    /// from inside a body, whose own type is not nameable there.
    pub struct MoveView<Fut = Ready<Result<()>>> {
        #[pin]
        body: Fut,
        done: bool,
    }
}

impl MoveView {
    /// Returns the future a body awaits to poll `view` in place.
    ///
    /// Each poll forwards to the view with the buffer the enclosing
    /// `MoveView` was polled with: first `poll_first`, whose content is
    /// reported back as the `MoveView`'s own, then `poll_swap` for every
    /// swap after it. The future stays pending after each report, so the
    /// view lives on in the body for the next poll, and resolves once the
    /// view has no further updates. An error the view produces is returned
    /// to the body.
    ///
    /// # Panics
    ///
    /// Panics when polled outside the poll of the `MoveView` whose body
    /// awaits it.
    #[doc(hidden)]
    pub fn drive<'cx, V>(cx: &'cx Cx, view: V) -> impl Future<Output = Result<()>> + use<'cx, V>
    where
        V: View,
    {
        DriveView {
            cx,
            view,
            first: true,
        }
    }
}

impl<Fut> MoveView<Fut>
where
    Fut: Future<Output = Result<()>>,
{
    #[doc(hidden)]
    pub fn new(body: Fut) -> Self {
        Self { body, done: false }
    }
}

impl<Fut> View for MoveView<Fut>
where
    Fut: Future<Output = Result<()>> + Send,
{
    fn poll_first(
        self: Pin<&mut Self>,
        _cx: &Cx,
        task: &mut Context<'_>,
        buf: &mut ViewBuffer,
    ) -> Poll<Result<ViewHandle>> {
        let this = self.project();
        let installed = Installed::new(buf);
        let poll = this.body.poll(task);
        match installed.finish() {
            Some(Response::First(content)) => {
                if poll.is_ready() {
                    *this.done = true;
                }
                Poll::Ready(Ok(content))
            }
            Some(Response::Swap(_)) => panic!("a `MoveView` body swapped before its first content"),
            None => match poll {
                Poll::Pending => Poll::Pending,
                // The body completed without driving a view; it renders
                // nothing and can never update.
                Poll::Ready(Ok(())) => {
                    *this.done = true;
                    Poll::Ready(Ok(ViewHandle::empty()))
                }
                Poll::Ready(Err(error)) => {
                    *this.done = true;
                    Poll::Ready(Err(error))
                }
            },
        }
    }

    fn poll_swap(
        self: Pin<&mut Self>,
        _cx: &Cx,
        task: &mut Context<'_>,
        buf: &mut ViewBuffer,
    ) -> Poll<Option<Result<Swap>>> {
        let this = self.project();
        if *this.done {
            return Poll::Ready(None);
        }
        let installed = Installed::new(buf);
        let poll = this.body.poll(task);
        match installed.finish() {
            Some(Response::Swap(swap)) => {
                if poll.is_ready() {
                    *this.done = true;
                }
                Poll::Ready(Some(Ok(swap)))
            }
            Some(Response::First(_)) => {
                panic!("`poll_swap` called before `poll_first` returned `Ready`")
            }
            None => match poll {
                Poll::Pending => Poll::Pending,
                Poll::Ready(Ok(())) => {
                    *this.done = true;
                    Poll::Ready(None)
                }
                Poll::Ready(Err(error)) => {
                    *this.done = true;
                    Poll::Ready(Some(Err(error)))
                }
            },
        }
    }
}

pin_project! {
    /// The future behind [`MoveView::drive`].
    struct DriveView<'cx, V> {
        cx: &'cx Cx,
        #[pin]
        view: V,
        first: bool,
    }
}

impl<V> Future for DriveView<'_, V>
where
    V: View,
{
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, task: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let mut taken = Taken::new();
        let exchange = taken.exchange();
        if *this.first {
            let content = ready!(this.view.poll_first(this.cx, task, &mut exchange.buffer))?;
            *this.first = false;
            exchange.response = Some(Response::First(content));
            Poll::Pending
        } else {
            match ready!(this.view.poll_swap(this.cx, task, &mut exchange.buffer)) {
                Some(Ok(swap)) => {
                    exchange.response = Some(Response::Swap(swap));
                    Poll::Pending
                }
                Some(Err(error)) => Poll::Ready(Err(error)),
                None => Poll::Ready(Ok(())),
            }
        }
    }
}
