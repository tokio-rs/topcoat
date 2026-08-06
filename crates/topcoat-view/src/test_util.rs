use std::{
    future::Future,
    pin::pin,
    task::{Context, Poll, Waker},
};

use topcoat_core::context::Cx;

use crate::{HtmlContext, PartsWriter, internal::__build_view, render::scope};

/// Builds a view inside a fresh scope through a writer sealed with `context`
/// and renders it.
pub(crate) fn render_with(
    context: HtmlContext,
    f: impl FnOnce(&Cx, &mut PartsWriter<'_>),
) -> String {
    block_on(scope(async {
        let cx = Cx::default();
        __build_view(|parts| parts.in_context(context, |parts| f(&cx, parts))).render(&cx)
    }))
}

/// Drives `fut` to completion on the current thread.
///
/// The futures under test never wait on external events, so polling in a
/// tight loop is sufficient.
pub(crate) fn block_on<F: Future>(fut: F) -> F::Output {
    let mut fut = pin!(fut);
    let mut cx = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(output) = fut.as_mut().poll(&mut cx) {
            return output;
        }
    }
}
