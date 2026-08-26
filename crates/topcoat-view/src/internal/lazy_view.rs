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
    /// A view built from the request context it is first polled with.
    ///
    /// The closure runs on the first poll with a copy of that context and
    /// returns the view polled in its place. A view built this way sees the
    /// context of whatever polls it, not the one in scope where it was
    /// constructed, so a caller can provide context to it by polling it
    /// under a derived context.
    #[project = LazyViewProj]
    pub enum LazyView<F, V> {
        Render { render: Option<F> },
        View { #[pin] view: V },
    }
}

impl<F, V> LazyView<F, V>
where
    F: FnOnce(Cx) -> V,
{
    #[must_use]
    pub fn new(render: F) -> Self {
        Self::Render {
            render: Some(render),
        }
    }
}

impl<F, V> View for LazyView<F, V>
where
    F: FnOnce(Cx) -> V + Send,
    V: View,
{
    fn poll_first(
        mut self: Pin<&mut Self>,
        cx: &Cx,
        task: &mut Context<'_>,
        buf: &mut ViewBuffer,
    ) -> Poll<Result<ViewHandle>> {
        loop {
            match self.as_mut().project() {
                LazyViewProj::Render { render } => {
                    let render = render
                        .take()
                        .expect("`poll_first` called again after it returned `Ready`");
                    let view = render(cx.clone()); // TODO maybe we dont need to clone here??
                    self.as_mut().set(Self::View { view });
                }
                LazyViewProj::View { view } => return view.poll_first(cx, task, buf),
            }
        }
    }

    fn poll_swap(
        self: Pin<&mut Self>,
        cx: &Cx,
        task: &mut Context<'_>,
        buf: &mut ViewBuffer,
    ) -> Poll<Option<Result<Swap>>> {
        match self.project() {
            LazyViewProj::View { view } => view.poll_swap(cx, task, buf),
            LazyViewProj::Render { .. } => {
                panic!("`poll_swap` called before `poll_first` returned `Ready`")
            }
        }
    }
}
