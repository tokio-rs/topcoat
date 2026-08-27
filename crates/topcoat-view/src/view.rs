use std::{
    fmt,
    future::poll_fn,
    pin::{Pin, pin},
    task::{Context, Poll, ready},
};

use futures_core::Stream;
use topcoat_core::error::Result;

use crate::buffer::ViewHandle;

/// The identity of a live region within a rendered view.
///
/// Displays as the number that marks the region's boundaries in the HTML.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RegionId(pub(crate) u64);

impl fmt::Display for RegionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

/// A replacement for the content of a live region, emitted after a view's
/// first content resolved.
#[derive(Debug)]
pub struct Swap {
    /// The region whose content is replaced.
    pub region: RegionId,
    /// The region's new content, self-contained: it renders without the
    /// buffer the view's first content was built in.
    pub replacement: ViewHandle,
}

/// What a poll of a [`View`] resolved to.
#[derive(Debug)]
pub enum Step {
    /// The view's first content: a [`ViewHandle`] pointing at the
    /// instruction block the view appended to its buffer.
    Content {
        content: ViewHandle,
        /// Whether the view may update after this content.
        live: bool,
    },
    /// A replacement for the content of one of the view's live regions.
    Swap {
        swap: Swap,
        /// Whether the view may update after this swap.
        live: bool,
    },
    /// The view has no further updates.
    Done,
}

/// A lazy view: an inert value that builds its content when polled.
///
/// A `view!` invocation evaluates to a value implementing this trait, and a
/// component returns one as `Result<impl View>`. Constructing a view does no
/// work; everything it writes happens inside [`poll`](View::poll), into the
/// [`ViewBuffer`](crate::ViewBuffer) the view was built with.
///
/// The first poll to resolve yields the view's first content. Every poll
/// after it yields an update one of the view's live regions emitted, until
/// the view reports it has no further updates.
pub trait View: Send {
    /// Polls the view toward its next step.
    ///
    /// The first `Ready` is always [`Step::Content`]; the view has appended
    /// its instruction block to its buffer and the handle points at it. The
    /// steps after it are [`Step::Swap`]s.
    ///
    /// A step's `live` flag tells whether polling on is worthwhile: `false`
    /// guarantees the view never updates again, while `true` leaves it to
    /// the next poll, which may still resolve to [`Step::Done`]. A view
    /// must not be polled again once it yielded `Done`, a step with `live`
    /// set to `false`, or an error.
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Step>>;
}

/// Combinators available on every [`View`].
///
/// Blanket implemented, so implementing [`View`] is enough to get them and an
/// implementation never has to care about them.
///
/// The handle a combinator resolves is self-contained when the view builds
/// in a buffer of its own, as a `view!` invocation naming its context does;
/// a view built against a shared buffer resolves a handle into that buffer,
/// which [`ViewHandle::seal`] makes self-contained.
pub trait ViewExt: View {
    /// Resolves the view's first content.
    ///
    /// Any updates the view would emit after its first content are
    /// discarded; [`single`](ViewExt::single) asserts there are none
    /// instead.
    fn first(self) -> impl Future<Output = Result<ViewHandle>> + Send
    where
        Self: Sized,
    {
        async move {
            let mut view = pin!(self);
            match poll_fn(|cx| view.as_mut().poll(cx)).await? {
                Step::Content { content, .. } => Ok(content),
                Step::Swap { .. } | Step::Done => panic!("{BEFORE_CONTENT}"),
            }
        }
    }

    /// Resolves the content of a view that never updates.
    ///
    /// Where [`first`](ViewExt::first) discards the updates a view emits
    /// after its first content, `single` asserts there are none: the view
    /// must report with its first content that it never updates. This is
    /// the method to reach for when a view is rendered once, into a
    /// fragment or a string.
    ///
    /// # Panics
    ///
    /// Panics if the view is live, that is, if it may update after its
    /// first content. Such a view is rendered with [`live`](ViewExt::live)
    /// instead.
    fn single(self) -> impl Future<Output = Result<ViewHandle>> + Send
    where
        Self: Sized,
    {
        async move {
            let mut view = pin!(self);
            match poll_fn(|cx| view.as_mut().poll(cx)).await? {
                Step::Content {
                    content,
                    live: false,
                } => Ok(content),
                Step::Content { live: true, .. } => panic!(
                    "`single` called on a live view, which may update after its first content; \
                     render it with `live` to receive the updates"
                ),
                Step::Swap { .. } | Step::Done => panic!("{BEFORE_CONTENT}"),
            }
        }
    }

    /// Resolves the view's first content and keeps the updates that follow.
    ///
    /// The stream beside the content yields a [`Swap`] for every live region
    /// that re-renders and ends once the view has no further updates.
    fn live(self) -> impl Future<Output = Result<(ViewHandle, Swaps<Self>)>> + Send
    where
        Self: Sized,
    {
        async move {
            let mut view = Box::pin(self);
            match poll_fn(|cx| view.as_mut().poll(cx)).await? {
                Step::Content { content, live } => Ok((content, Swaps { view, live })),
                Step::Swap { .. } | Step::Done => panic!("{BEFORE_CONTENT}"),
            }
        }
    }

    /// Erases the view's concrete type behind a boxed one.
    ///
    /// Every `view!` invocation has its own anonymous type, so a function
    /// returning `impl View` from multiple `return` sites must box each view
    /// to give them a common type.
    fn boxed<'a>(self) -> BoxView<'a>
    where
        Self: Sized + 'a,
    {
        Box::pin(self)
    }
}

impl<V: View + ?Sized> ViewExt for V {}

const BEFORE_CONTENT: &str = "a view swapped or completed before its first content";

/// The updates a view emits after its first content, returned by
/// [`ViewExt::live`].
///
/// Yields a [`Swap`] for every live region that re-renders and ends once the
/// view has no further updates.
pub struct Swaps<V> {
    view: Pin<Box<V>>,
    /// Whether the view may still update; `false` once it reported it never
    /// will, so it is not polled again.
    live: bool,
}

impl<V> Swaps<V> {
    /// Whether the view may still emit updates.
    ///
    /// Once this is `false`, the stream is exhausted: the view reported it
    /// never updates again.
    #[must_use]
    pub fn is_live(&self) -> bool {
        self.live
    }
}

impl<V: View> Stream for Swaps<V> {
    type Item = Result<Swap>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        if !this.live {
            return Poll::Ready(None);
        }
        match ready!(this.view.as_mut().poll(cx)) {
            Ok(Step::Swap { swap, live }) => {
                this.live = live;
                Poll::Ready(Some(Ok(swap)))
            }
            Ok(Step::Done) => {
                this.live = false;
                Poll::Ready(None)
            }
            Ok(Step::Content { .. }) => panic!("a view produced content twice"),
            Err(error) => {
                this.live = false;
                Poll::Ready(Some(Err(error)))
            }
        }
    }
}

impl View for () {
    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<Step>> {
        Poll::Ready(Ok(Step::Content {
            content: ViewHandle::empty(),
            live: false,
        }))
    }
}

pub type BoxView<'a> = Pin<Box<dyn View + 'a>>;

impl View for BoxView<'_> {
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Step>> {
        self.get_mut().as_mut().poll(cx)
    }
}
