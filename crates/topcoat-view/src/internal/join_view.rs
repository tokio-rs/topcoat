use std::{
    pin::Pin,
    task::{Context, Poll, ready},
};

use pin_project_lite::pin_project;
use topcoat_core::{context::Cx, error::Result};

use super::Builder;
use crate::{View, ViewFirst, ViewHandle, ViewSwap};

pin_project! {
    /// A [`View`] that renders dynamic nodes concurrently.
    ///
    /// Once every node has initial content, `burst` writes the template's
    /// instruction block synchronously, inserting content in source order.
    /// Later updates are polled round-robin so a busy node cannot starve
    /// its siblings.
    pub struct JoinView<'cx, U, F> {
        cx: &'cx Cx,
        #[pin]
        units: U,
        // The burst; taken when the units resolve.
        burst: Option<F>,
        // Where the next swap scan starts. Advanced past each unit that
        // delivered a swap, so its siblings get a turn before it comes up
        // again.
        next_swap_index: usize,
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
            next_swap_index: 0,
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
        let mut this = self.project();

        // One full turn around the ring in two range scans, so every
        // waiting unit is polled before this view settles on pending.
        let len = U::LEN;
        let start = *this.next_swap_index;
        let mut all_done = true;
        for (from, to) in [(start, len), (0, start)] {
            match this.units.as_mut().poll_swap_range(cx, from, to) {
                Poll::Pending => all_done = false,
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Ready(Ok(None)) => {}
                Poll::Ready(Ok(Some((position, swap)))) => {
                    *this.next_swap_index = (position + 1) % len;
                    return Poll::Ready(Ok(Some(swap)));
                }
            }
        }

        if all_done {
            Poll::Ready(Ok(None))
        } else {
            Poll::Pending
        }
    }
}

/// A set of views polled concurrently by [`JoinView`].
///
/// A [`JoinUnit`] holds one view, and a [`JoinPair`] joins two sets in
/// position order. `()` is the empty set. Resolved contents mirror the
/// structure: a `ViewHandle` for each unit, a pair for each [`JoinPair`].
///
/// Pairing sets into a balanced tree keeps the type's nesting depth
/// logarithmic in the number of views, so trait checks on large templates
/// stay within the compiler's recursion limit.
pub trait JoinUnits {
    /// The number of units in the set.
    const LEN: usize;

    /// The units' contents, in the same structure as the set.
    type Contents;

    /// Polls pending units until all have initial content or one fails.
    fn poll_contents(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>>;

    /// Takes the resolved contents out of the units.
    fn take_contents(self: Pin<&mut Self>) -> Self::Contents;

    /// Whether any unit may still update.
    fn is_live(&self) -> bool;

    /// Polls units in `from..to` for an update and returns its position.
    /// Returns `None` when every unit in the range has finished.
    fn poll_swap_range(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        from: usize,
        to: usize,
    ) -> Poll<Result<Option<(usize, ViewSwap)>>>;
}

impl JoinUnits for () {
    const LEN: usize = 0;

    type Contents = ();

    fn poll_contents(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn take_contents(self: Pin<&mut Self>) -> Self::Contents {}

    fn is_live(&self) -> bool {
        false
    }

    fn poll_swap_range(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _from: usize,
        _to: usize,
    ) -> Poll<Result<Option<(usize, ViewSwap)>>> {
        Poll::Ready(Ok(None))
    }
}

pin_project! {
    /// One dynamic position of a [`JoinView`].
    pub struct JoinUnit<V> {
        #[pin]
        view: V,
        // The unit's content, held until the burst takes it.
        content: Option<ViewHandle>,
        // Whether the view reported it has no further updates.
        done: bool,
    }
}

impl<V> JoinUnit<V>
where
    V: View,
{
    #[must_use]
    pub fn new(view: V) -> Self {
        Self {
            view,
            content: None,
            done: false,
        }
    }
}

impl<V> JoinUnits for JoinUnit<V>
where
    V: View,
{
    const LEN: usize = 1;

    type Contents = ViewHandle;

    fn poll_contents(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        let this = self.project();
        if this.content.is_some() {
            return Poll::Ready(Ok(()));
        }

        let ViewFirst { content, live } = ready!(this.view.poll_first(cx))?;
        *this.content = Some(content);
        *this.done = !live;
        Poll::Ready(Ok(()))
    }

    fn take_contents(self: Pin<&mut Self>) -> Self::Contents {
        self.project()
            .content
            .take()
            .expect("every unit resolved its content")
    }

    fn is_live(&self) -> bool {
        !self.done
    }

    fn poll_swap_range(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        from: usize,
        to: usize,
    ) -> Poll<Result<Option<(usize, ViewSwap)>>> {
        let this = self.project();
        if from > 0 || to == 0 || *this.done {
            return Poll::Ready(Ok(None));
        }

        let swap = ready!(this.view.poll_swap(cx))?;
        if swap.is_none() {
            *this.done = true;
        }
        Poll::Ready(Ok(swap.map(|swap| (0, swap))))
    }
}

pin_project! {
    /// Two sets of [`JoinView`] units, the `left` positions before the
    /// `right` ones.
    pub struct JoinPair<L, R> {
        #[pin]
        left: L,
        #[pin]
        right: R,
    }
}

impl<L, R> JoinPair<L, R>
where
    L: JoinUnits,
    R: JoinUnits,
{
    #[must_use]
    pub fn new(left: L, right: R) -> Self {
        Self { left, right }
    }
}

impl<L, R> JoinUnits for JoinPair<L, R>
where
    L: JoinUnits,
    R: JoinUnits,
{
    const LEN: usize = L::LEN + R::LEN;

    type Contents = (L::Contents, R::Contents);

    fn poll_contents(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        let this = self.project();
        // Both sides are polled even if one is pending, so every unit makes
        // progress on each poll.
        let left = this.left.poll_contents(cx)?;
        let right = this.right.poll_contents(cx)?;
        ready!(left);
        right.map(Ok)
    }

    fn take_contents(self: Pin<&mut Self>) -> Self::Contents {
        let this = self.project();
        (this.left.take_contents(), this.right.take_contents())
    }

    fn is_live(&self) -> bool {
        self.left.is_live() || self.right.is_live()
    }

    fn poll_swap_range(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        from: usize,
        to: usize,
    ) -> Poll<Result<Option<(usize, ViewSwap)>>> {
        let this = self.project();
        let split = L::LEN;
        let mut pending = false;

        if from < split {
            match this.left.poll_swap_range(cx, from, to.min(split))? {
                Poll::Ready(Some(swap)) => return Poll::Ready(Ok(Some(swap))),
                Poll::Ready(None) => {}
                Poll::Pending => pending = true,
            }
        }

        if to > split {
            match this
                .right
                .poll_swap_range(cx, from.saturating_sub(split), to - split)?
            {
                Poll::Ready(Some((position, swap))) => {
                    return Poll::Ready(Ok(Some((position + split, swap))));
                }
                Poll::Ready(None) => {}
                Poll::Pending => pending = true,
            }
        }

        if pending {
            Poll::Pending
        } else {
            Poll::Ready(Ok(None))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::task::Waker;

    use topcoat_core::identity::{Identity, SiteKey};

    use super::*;
    use crate::RegionId;

    /// A live view that delivers one swap for its region per poll, a fixed
    /// number of times.
    struct Ticker {
        region: RegionId,
        remaining: usize,
    }

    impl View for Ticker {
        fn poll_first(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<ViewFirst>> {
            Poll::Ready(Ok(ViewFirst {
                content: ViewHandle::empty(),
                live: true,
            }))
        }

        fn poll_swap(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<Option<ViewSwap>>> {
            let this = self.get_mut();
            if this.remaining == 0 {
                return Poll::Ready(Ok(None));
            }
            this.remaining -= 1;
            Poll::Ready(Ok(Some(ViewSwap {
                region: this.region,
                replacement: ViewHandle::empty(),
            })))
        }
    }

    #[test]
    fn swaps_are_collected_round_robin() {
        let [a, b, c] = [0, 1, 2].map(|ordinal| {
            RegionId::new(
                Identity::ROOT,
                SiteKey::new(file!(), line!(), column!(), ordinal),
            )
        });

        let ticker = |region, remaining| Ticker { region, remaining };
        let units = JoinPair::new(
            JoinUnit::new(ticker(a, 3)),
            JoinPair::new(JoinUnit::new(ticker(b, 1)), JoinUnit::new(ticker(c, 2))),
        );
        let cx = Cx::new();
        let mut view = std::pin::pin!(JoinView::new(&cx, units, |_, _| ()));

        let mut task_cx = Context::from_waker(Waker::noop());
        let mut order = Vec::new();
        loop {
            match view.as_mut().poll_swap(&mut task_cx) {
                Poll::Ready(Ok(Some(swap))) => order.push(swap.region),
                Poll::Ready(Ok(None)) => break,
                Poll::Ready(Err(_)) => panic!("the tickers never fail"),
                Poll::Pending => panic!("the tickers are always ready"),
            }
        }

        // Every unit gets a turn between two swaps of a busier sibling.
        assert_eq!(order, [a, b, c, a, c, a]);
    }
}
