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
    /// order. After that the units' updates merge into one stream of swaps,
    /// collected round-robin so one busy unit cannot starve its siblings.
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

/// The units a [`JoinView`] drives: one level of the template's position
/// list.
///
/// The positions build different view types, so the expansion nests a
/// [`JoinUnit`] per position, terminated by `()`. The contents come back in
/// the same nested shape, destructured by the burst.
pub trait JoinUnits {
    /// The number of units in the list.
    const LEN: usize;

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

    /// Polls the units at positions `from..to` for the next swap, yielded
    /// with its position, or for `None` once every unit in the range has no
    /// further updates.
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
    const LEN: usize = 1 + Rest::LEN;

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

    fn poll_swap_range(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        from: usize,
        to: usize,
    ) -> Poll<Result<Option<(usize, ViewSwap)>>> {
        if to == 0 {
            return Poll::Ready(Ok(None));
        }

        let this = self.project();
        let mut pending = false;

        if from == 0 && !*this.done {
            match this.view.poll_swap(cx) {
                Poll::Ready(Ok(Some(swap))) => return Poll::Ready(Ok(Some((0, swap)))),
                Poll::Ready(Ok(None)) => *this.done = true,
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => pending = true,
            }
        }

        match this
            .rest
            .poll_swap_range(cx, from.saturating_sub(1), to - 1)
        {
            Poll::Ready(Ok(Some((position, swap)))) => Poll::Ready(Ok(Some((position + 1, swap)))),
            // The rest of the range is done, but this unit still owes a swap.
            Poll::Ready(Ok(None)) if pending => Poll::Pending,
            other => other,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::task::Waker;

    use super::*;
    use crate::{RegionId, region::RegionScope};

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
        let mut counter = 1;
        let _regions = RegionScope::new(&mut counter);
        let (a, b, c) = (RegionId::next(), RegionId::next(), RegionId::next());

        let ticker = |region, remaining| Ticker { region, remaining };
        let units = JoinUnit::new(
            ticker(a, 3),
            JoinUnit::new(ticker(b, 1), JoinUnit::new(ticker(c, 2), ())),
        );
        let cx = Cx::default();
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
