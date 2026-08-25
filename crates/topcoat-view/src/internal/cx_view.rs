use std::{
    pin::Pin,
    task::{Context, Poll},
};

use pin_project_lite::pin_project;
use topcoat_core::{context::Cx, error::Result};

use crate::{
    Swap, View,
    buffer::{ViewBuffer, ViewHandle},
};

pin_project! {
    /// A view rendered against its own request context.
    ///
    /// `view! { cx => ... }` wraps its template in this type: the named
    /// context is owned by the view, and every poll passes it down in place
    /// of the caller's context, so everything beneath the invocation sees
    /// the named context.
    pub struct CxView<V> {
        cx: Cx,
        #[pin]
        view: V,
    }
}

impl<V> CxView<V>
where
    V: View,
{
    #[must_use]
    pub fn new(cx: Cx, view: V) -> Self {
        Self { cx, view }
    }
}

impl<V> View for CxView<V>
where
    V: View,
{
    fn poll_first(
        self: Pin<&mut Self>,
        _cx: &Cx,
        task: &mut Context<'_>,
        buf: &mut ViewBuffer,
    ) -> Poll<Result<ViewHandle>> {
        let this = self.project();
        this.view.poll_first(this.cx, task, buf)
    }

    fn poll_swap(
        self: Pin<&mut Self>,
        _cx: &Cx,
        task: &mut Context<'_>,
        buf: &mut ViewBuffer,
    ) -> Poll<Option<Result<Swap>>> {
        let this = self.project();
        this.view.poll_swap(this.cx, task, buf)
    }
}
