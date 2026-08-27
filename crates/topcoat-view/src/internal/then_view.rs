use std::{
    pin::Pin,
    task::{Context, Poll, ready},
};

use pin_project_lite::pin_project;
use topcoat_core::error::Result;

use crate::{Step, View};

pin_project! {
    /// The view a future resolves to.
    ///
    /// Adapts a future resolving to a view, a component's render future or
    /// any async work that runs before a template, into a [`View`]. The
    /// future is driven first; the view it resolves to is then polled in
    /// its place.
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
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Step>> {
        loop {
            match self.as_mut().project() {
                ThenViewProj::Future { future } => {
                    let view = ready!(future.poll(cx))?;
                    self.as_mut().set(Self::View { view });
                }
                ThenViewProj::View { view } => return view.poll(cx),
            }
        }
    }
}
