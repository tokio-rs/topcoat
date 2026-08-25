use std::{
    cell::Cell,
    pin::Pin,
    sync::atomic::{AtomicU64, Ordering},
    task::{Context, Poll, ready},
};

use pin_project_lite::pin_project;
use topcoat_core::{
    context::Cx,
    error::{Error, Result},
};

use crate::{
    RegionId, Swap, View,
    buffer::{ViewBuffer, ViewHandle},
};

/// The id of the next live region.
///
/// A process-wide counter, so every live region in a response is distinct
/// without threading state through the request.
static NEXT_REGION: AtomicU64 = AtomicU64::new(1);

thread_local! {
    /// The emission in flight on the current task, if any: placed by the
    /// [`EmitView`] being polled, taken by the enclosing [`LiveView`]'s
    /// [`collect`] around the poll.
    static YIELD: Cell<Option<Emission>> = const { Cell::new(None) };
}

/// One item tunneled from an `emit!` invocation to the enclosing `live!`
/// region.
enum Emission {
    /// The self-contained content of one emit call. The enclosing region
    /// packages it: as its first content in `poll_first`, or as a swap
    /// replacing that content in `poll_swap`.
    Content(ViewHandle),
    /// A swap from a nested live region, passed through verbatim.
    Swap(Swap),
}

/// Moves the emission into the task's slot if it is free.
///
/// The slot is occupied when another emission is still waiting to be
/// collected; the caller stays pending and tries again when polled.
fn try_yield(emission: &mut Option<Emission>) {
    let current = YIELD.take();
    if current.is_some() {
        YIELD.set(current);
        return;
    }
    YIELD.set(emission.take());
}

/// Restores the emission an enclosing [`collect`] had in flight, also when
/// the collected poll panics.
struct Collect {
    previous: Option<Emission>,
}

impl Drop for Collect {
    fn drop(&mut self) {
        YIELD.set(self.previous.take());
    }
}

/// Polls `f` and takes the emission it tunneled, if any.
///
/// An emission already in flight from an enclosing collect is parked for the
/// duration of the poll, so nested live regions collect only their own
/// body's emissions.
fn collect<F>(f: Pin<&mut F>, task: &mut Context<'_>) -> (Poll<F::Output>, Option<Emission>)
where
    F: Future + ?Sized,
{
    let _guard = Collect {
        previous: YIELD.take(),
    };
    let poll = f.poll(task);
    let emission = YIELD.take();
    (poll, emission)
}

pin_project! {
    /// A live region: a node position whose content is replaced by the views
    /// its body emits.
    ///
    /// The `live!` macro wraps its body in this type. The body emits with
    /// `emit!`: the first emission becomes the region's content, surrounded
    /// by marker comments, and every later one becomes a [`Swap`] replacing
    /// that content on the client.
    pub struct LiveView<Fut> {
        #[pin]
        body: Fut,
        // The region's id, decided at the first poll.
        region: Option<RegionId>,
        // An error the body completed with while an emission was still in
        // flight; yielded through `poll_swap` after it.
        error: Option<Error>,
        done: bool,
    }
}

impl<Fut> LiveView<Fut>
where
    Fut: Future<Output = Result<()>>,
{
    #[doc(hidden)]
    pub fn new(body: Fut) -> Self {
        Self {
            body,
            region: None,
            error: None,
            done: false,
        }
    }
}

impl<Fut> View for LiveView<Fut>
where
    Fut: Future<Output = Result<()>> + Send,
{
    fn poll_first(
        self: Pin<&mut Self>,
        cx: &Cx,
        task: &mut Context<'_>,
        buf: &mut ViewBuffer,
    ) -> Poll<Result<ViewHandle>> {
        let this = self.project();
        let region = *this
            .region
            .get_or_insert_with(|| RegionId(NEXT_REGION.fetch_add(1, Ordering::Relaxed)));
        let (poll, emitted) = collect(this.body, task);
        if let Some(emission) = emitted {
            match poll {
                Poll::Ready(Ok(())) => *this.done = true,
                Poll::Ready(Err(error)) => *this.error = Some(error),
                Poll::Pending => {}
            }
            return match emission {
                Emission::Content(content) => {
                    let view = buf.block(cx, |b| {
                        b.parts()
                            .push_str_unescaped(&format!("<!--tc:{}-->", region.0));
                        b.view(content);
                        b.parts()
                            .push_str_unescaped(&format!("<!--/tc:{}-->", region.0));
                    });
                    Poll::Ready(Ok(view))
                }
                Emission::Swap(_) => {
                    panic!("a live region emitted a swap before its first content")
                }
            };
        }
        match poll {
            Poll::Pending => Poll::Pending,
            // The body completed without emitting; the region renders
            // nothing and can never update, so no markers are written.
            Poll::Ready(Ok(())) => {
                *this.done = true;
                Poll::Ready(Ok(ViewHandle::empty()))
            }
            Poll::Ready(Err(error)) => {
                *this.done = true;
                Poll::Ready(Err(error))
            }
        }
    }

    fn poll_swap(
        self: Pin<&mut Self>,
        _cx: &Cx,
        task: &mut Context<'_>,
        _buf: &mut ViewBuffer,
    ) -> Poll<Option<Result<Swap>>> {
        let this = self.project();
        if let Some(error) = this.error.take() {
            *this.done = true;
            return Poll::Ready(Some(Err(error)));
        }
        if *this.done {
            return Poll::Ready(None);
        }
        let region = this
            .region
            .expect("`poll_swap` called before `poll_first` returned `Ready`");
        let (poll, emitted) = collect(this.body, task);
        if let Some(emission) = emitted {
            match poll {
                Poll::Ready(Ok(())) => *this.done = true,
                Poll::Ready(Err(error)) => *this.error = Some(error),
                Poll::Pending => {}
            }
            return Poll::Ready(Some(Ok(match emission {
                Emission::Content(replacement) => Swap {
                    region,
                    replacement,
                },
                // A nested region's swap targets its own region; it passes
                // through untouched.
                Emission::Swap(swap) => swap,
            })));
        }
        match poll {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(())) => {
                *this.done = true;
                Poll::Ready(None)
            }
            Poll::Ready(Err(error)) => {
                *this.done = true;
                Poll::Ready(Some(Err(error)))
            }
        }
    }
}

pin_project! {
    /// One emission of a live region's body: the future an `emit!`
    /// invocation is awaited as.
    ///
    /// Drives the emitted view to its content in a buffer of its own, so the
    /// content is self-contained, and tunnels it to the enclosing region.
    /// After that it keeps driving the view's own swaps — nested live
    /// regions — tunneling each one through verbatim. It resolves once the
    /// view has no further updates; an error the view produces is returned
    /// to the caller instead of failing the region, and nothing is tunneled
    /// for it.
    pub struct EmitView<'cx, V> {
        cx: &'cx Cx,
        #[pin]
        view: V,
        // The buffer the content is built in until it is sealed, then a
        // scratch buffer for the swap phase.
        buffer: Option<ViewBuffer>,
        first: bool,
        // An emission waiting for the tunnel's slot to be free.
        pending: Option<Emission>,
    }
}

impl<'cx, V> EmitView<'cx, V>
where
    V: View,
{
    #[doc(hidden)]
    pub fn new(cx: &'cx Cx, view: V) -> Self {
        Self {
            cx,
            view,
            buffer: None,
            first: true,
            pending: None,
        }
    }
}

impl<V> Future for EmitView<'_, V>
where
    V: View,
{
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, task: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        loop {
            if this.pending.is_some() {
                try_yield(this.pending);
                // Placed or not, the emission awaits collection by the
                // enclosing region's poll; resume when polled again.
                return Poll::Pending;
            }
            let buffer = this.buffer.get_or_insert_with(ViewBuffer::new);
            if *this.first {
                let content = ready!(this.view.as_mut().poll_first(this.cx, task, buffer))?;
                *this.first = false;
                let buffer = this
                    .buffer
                    .take()
                    .expect("the content was built in the buffer");
                *this.pending = Some(Emission::Content(content.seal(buffer)));
            } else {
                match ready!(this.view.as_mut().poll_swap(this.cx, task, buffer)) {
                    Some(Ok(swap)) => *this.pending = Some(Emission::Swap(swap)),
                    Some(Err(error)) => return Poll::Ready(Err(error)),
                    None => return Poll::Ready(Ok(())),
                }
            }
        }
    }
}
