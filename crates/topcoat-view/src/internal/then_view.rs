use std::{
    pin::Pin,
    task::{Context, Poll, ready},
};

use pin_project_lite::pin_project;
use topcoat_core::error::Result;

use crate::{Swap, View, buffer::ViewHandle};

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
    fn poll_first(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<ViewHandle>> {
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

    fn poll_swap(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Swap>>> {
        match self.project() {
            ThenViewProj::Future { .. } => {
                panic!("`poll_swap` called before `poll_first` returned `Ready`")
            }
            ThenViewProj::View { view } => view.poll_swap(cx),
        }
    }
}
