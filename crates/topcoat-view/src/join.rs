use std::{
    future::poll_fn,
    ops::DerefMut,
    pin::Pin,
    task::{Context, Poll},
};

use futures_core::Stream;
use topcoat_core::error::Result;

use crate::{ViewChunk, ViewHandle, yielder::collect};

/// Drives the dynamic positions of a template concurrently.
///
/// Holds a tuple of [`Unit`]s, or a `Vec` of them for a loop body.
/// [`first`](Join::first) resolves every unit's content — what renders in the
/// unit's position — and the join then remains a stream of the swaps the
/// units emit after that, each targeting its own position.
pub struct Join<T> {
    units: T,
}

impl<T: JoinUnits> Join<T> {
    /// Joins the given units.
    #[must_use]
    pub fn new(units: T) -> Self {
        Self { units }
    }

    /// Resolves every unit's content, in unit order.
    ///
    /// Drives the units concurrently until each has emitted its content or
    /// completed; a unit completing without emitting resolves to `None`. The
    /// first unit to fail fails the join, dropping the other units' progress.
    ///
    /// Must be called once, before streaming the join's remaining chunks.
    pub async fn first(&mut self) -> Result<T::First> {
        poll_fn(|cx| self.units.poll_first(cx)).await?;
        Ok(self.units.take_first())
    }
}

/// The chunks the units emit after their content, merged.
///
/// A unit's error ends that unit and is yielded in-band; the other units
/// keep streaming. The stream ends when every unit has completed.
impl<T: JoinUnits + Unpin> Stream for Join<T> {
    type Item = Result<ViewChunk>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.get_mut().units.poll_next(cx)
    }
}

/// One position driven by a [`Join`]: its future, and the content it emitted
/// while it waits for [`Join::first`] to hand it out.
///
/// The pointer type `P` is `&mut F` for a future the caller pinned in place,
/// or `Box<F>` for one of a loop body's iterations.
pub struct Unit<P> {
    /// The unit's future; `None` once it completed.
    future: Option<Pin<P>>,
    /// The unit's content, until [`Join::first`] takes it.
    content: Option<ViewHandle>,
}

impl<P> Unit<P>
where
    P: DerefMut,
    P::Target: Future<Output = Result<()>>,
{
    /// Wraps a pinned future as a join unit.
    #[must_use]
    pub fn new(future: Pin<P>) -> Self {
        Self {
            future: Some(future),
            content: None,
        }
    }

    /// Polls toward the unit's content; ready once it is emitted or the
    /// future completed.
    ///
    /// A unit that emitted its content is parked — not polled again — until
    /// the stream phase resumes it; its pending yield needs no waker, only a
    /// later poll.
    fn poll_first(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>> {
        if self.content.is_some() {
            return Poll::Ready(Ok(()));
        }
        let Some(future) = self.future.as_mut() else {
            return Poll::Ready(Ok(()));
        };
        let (poll, emitted) = collect(future.as_mut(), cx);
        match poll {
            Poll::Pending => {}
            Poll::Ready(Ok(())) => self.future = None,
            Poll::Ready(Err(error)) => {
                self.future = None;
                return Poll::Ready(Err(error));
            }
        }
        match emitted {
            Some(Ok(ViewChunk::Content(view))) => {
                self.content = Some(view);
                Poll::Ready(Ok(()))
            }
            Some(Ok(ViewChunk::Swap { .. })) => {
                panic!("a joined position emitted a swap before its content")
            }
            Some(Err(error)) => {
                self.future = None;
                Poll::Ready(Err(error))
            }
            None if self.future.is_some() => Poll::Pending,
            None => Poll::Ready(Ok(())),
        }
    }

    /// Polls for a chunk emitted after the content; ready with `None` once
    /// the future completed.
    fn poll_tail(&mut self, cx: &mut Context<'_>) -> Poll<Option<Result<ViewChunk>>> {
        let Some(future) = self.future.as_mut() else {
            return Poll::Ready(None);
        };
        let (poll, emitted) = collect(future.as_mut(), cx);
        match poll {
            Poll::Pending => {}
            Poll::Ready(Ok(())) => self.future = None,
            Poll::Ready(Err(error)) => {
                self.future = None;
                return Poll::Ready(Some(Err(error)));
            }
        }
        match emitted {
            Some(item) => Poll::Ready(Some(item)),
            None if self.future.is_some() => Poll::Pending,
            None => Poll::Ready(None),
        }
    }
}

/// The [`Unit`]s a [`Join`] drives: a tuple of them, or a `Vec` for a loop
/// body.
pub trait JoinUnits {
    /// The units' content, in unit order.
    type First;

    /// Polls every unit still waiting on its content; ready when all have
    /// resolved, or with the first unit's error.
    fn poll_first(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>>;

    /// Takes the resolved content out of the units.
    fn take_first(&mut self) -> Self::First;

    /// Polls the units for a chunk emitted after their content; ready with
    /// `None` once every unit completed.
    fn poll_next(&mut self, cx: &mut Context<'_>) -> Poll<Option<Result<ViewChunk>>>;
}

impl<P> JoinUnits for Vec<Unit<P>>
where
    P: DerefMut,
    P::Target: Future<Output = Result<()>>,
{
    type First = Vec<Option<ViewHandle>>;

    fn poll_first(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>> {
        let mut ready = true;
        for unit in self {
            match unit.poll_first(cx) {
                Poll::Ready(Ok(())) => {}
                Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                Poll::Pending => ready = false,
            }
        }
        if ready {
            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
        }
    }

    fn take_first(&mut self) -> Self::First {
        self.iter_mut().map(|unit| unit.content.take()).collect()
    }

    fn poll_next(&mut self, cx: &mut Context<'_>) -> Poll<Option<Result<ViewChunk>>> {
        let mut pending = false;
        for unit in self {
            match unit.poll_tail(cx) {
                Poll::Ready(Some(item)) => return Poll::Ready(Some(item)),
                Poll::Ready(None) => {}
                Poll::Pending => pending = true,
            }
        }
        if pending {
            Poll::Pending
        } else {
            Poll::Ready(None)
        }
    }
}

macro_rules! first_ty {
    ($P:ident) => { Option<ViewHandle> };
}

macro_rules! impl_join_units {
    ($($P:ident),+) => {
        impl<$($P),+> JoinUnits for ($(Unit<$P>,)+)
        where
            $($P: DerefMut, $P::Target: Future<Output = Result<()>>,)+
        {
            type First = ($(first_ty!($P),)+);

            #[allow(non_snake_case)]
            fn poll_first(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>> {
                let ($($P,)+) = self;
                let mut ready = true;
                $(match $P.poll_first(cx) {
                    Poll::Ready(Ok(())) => {}
                    Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                    Poll::Pending => ready = false,
                })+
                if ready {
                    Poll::Ready(Ok(()))
                } else {
                    Poll::Pending
                }
            }

            #[allow(non_snake_case)]
            fn take_first(&mut self) -> Self::First {
                let ($($P,)+) = self;
                ($($P.content.take(),)+)
            }

            #[allow(non_snake_case)]
            fn poll_next(&mut self, cx: &mut Context<'_>) -> Poll<Option<Result<ViewChunk>>> {
                let ($($P,)+) = self;
                let mut pending = false;
                $(match $P.poll_tail(cx) {
                    Poll::Ready(Some(item)) => return Poll::Ready(Some(item)),
                    Poll::Ready(None) => {}
                    Poll::Pending => pending = true,
                })+
                if pending {
                    Poll::Pending
                } else {
                    Poll::Ready(None)
                }
            }
        }
    };
}

impl_join_units!(P1);
impl_join_units!(P1, P2);
impl_join_units!(P1, P2, P3);
impl_join_units!(P1, P2, P3, P4);
impl_join_units!(P1, P2, P3, P4, P5);
impl_join_units!(P1, P2, P3, P4, P5, P6);
impl_join_units!(P1, P2, P3, P4, P5, P6, P7);
impl_join_units!(P1, P2, P3, P4, P5, P6, P7, P8);
impl_join_units!(P1, P2, P3, P4, P5, P6, P7, P8, P9);
impl_join_units!(P1, P2, P3, P4, P5, P6, P7, P8, P9, P10);
impl_join_units!(P1, P2, P3, P4, P5, P6, P7, P8, P9, P10, P11);
impl_join_units!(P1, P2, P3, P4, P5, P6, P7, P8, P9, P10, P11, P12);
