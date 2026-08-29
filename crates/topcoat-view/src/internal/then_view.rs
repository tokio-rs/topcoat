use std::{
    pin::Pin,
    task::{Context, Poll, ready},
};

use pin_project_lite::pin_project;
use topcoat_core::error::Result;

use crate::{View, ViewFirst, ViewSwap};

pin_project! {
    /// A [`View`] built from a [`Future`] that resolves to a [`View`].
    ///
    /// This view first awaits the future for its returned view, and then adopts
    /// then re-emits all values from that view.
    ///
    /// This is useful, for example, for components, which are async functions that return
    /// a view. The parent caller needs to convert their future into a new view to integrate.
    #[project = ThenViewProj]
    pub enum ThenView<F, V> {
        Future { #[pin] future: F },
        View { #[pin] view: V },
    }
}

impl<F, V> ThenView<F, V>
where
    F: Future<Output = Result<V>>,
{
    #[must_use]
    pub fn new(future: F) -> Self {
        Self::Future { future }
    }
}

impl<F, V> View for ThenView<F, V>
where
    F: Future<Output = Result<V>> + Send,
    V: View,
{
    fn poll_first(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<ViewFirst>> {
        loop {
            match self.as_mut().project() {
                ThenViewProj::Future { future } => {
                    let view = ready!(future.poll(cx))?;
                    self.as_mut().set(Self::View { view });
                }
                ThenViewProj::View { view } => return view.poll_first(cx),
            }
        }
    }

    fn poll_swap(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Option<ViewSwap>>> {
        loop {
            match self.as_mut().project() {
                ThenViewProj::Future { .. } => {
                    panic!(
                        "called `.poll_swap` on a `ThenView` that has not yet emitted any `First` content"
                    );
                }
                ThenViewProj::View { view } => return view.poll_swap(cx),
            }
        }
    }
}
