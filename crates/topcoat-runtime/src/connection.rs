use std::any::TypeId;

use topcoat_core::context::{Cx, try_request_context};
use topcoat_view::{HoistKey, hoist_once};

/// Marks a request as a render over an open browser connection.
///
/// Register this in the request context of a render driven by a connection.
/// Renders over plain HTTP do not carry it.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct ConnectedRender;

/// Whether the current render runs over a browser connection, requesting
/// one when it does not.
///
/// Over HTTP this returns `false` and marks the enclosing page or shard as
/// needing a connection once the response finishes. The browser then
/// connects and renders that page or shard again, where this returns
/// `true`. Calling it opts into a connection even when the result only
/// selects some text, so use [`connected_untracked`] to adapt without
/// requesting one.
///
/// The result describes the current render, not whether another part of
/// the document has a connection. The requirement belongs to the innermost
/// enclosing page or shard; a call inside a layout or component belongs to
/// the page rendering it.
///
/// # Panics
///
/// Panics outside an active rendering scope. A spawned task must establish
/// its own rendering scope before calling this.
#[must_use]
#[track_caller]
pub fn connected(cx: &Cx) -> bool {
    hoist_once(HoistKey::new(TypeId::of::<ConnectedRender>()), |parts| {
        parts.push_comment(|comment| {
            comment.push_promoted_str_unescaped(&"::topcoat::connect");
        });
    });
    connected_untracked(cx)
}

/// Whether the current render runs over a browser connection, without
/// requesting one.
///
/// This lets a component adapt to a connected render without making every
/// page using it open a connection. It works in any context and never
/// renders anything.
#[must_use]
pub fn connected_untracked(cx: &Cx) -> bool {
    try_request_context::<ConnectedRender>(cx).is_some()
}

#[cfg(test)]
mod tests {
    use std::{
        panic::{AssertUnwindSafe, catch_unwind},
        pin::pin,
        task::{Context, Poll, Waker},
    };

    use topcoat::view::{HoistView, ViewExt, internal::ThenView, view};

    use super::*;

    const MARKER: &str = "<!--::topcoat::connect-->";

    /// Drives a future that never yields to completion.
    fn block_on<F: Future>(future: F) -> F::Output {
        let mut future = pin!(future);
        let mut cx = Context::from_waker(Waker::noop());
        loop {
            if let Poll::Ready(output) = future.as_mut().poll(&mut cx) {
                return output;
            }
        }
    }

    /// Renders a body under `cx` whose content shows what `check` returned.
    fn render(cx: Cx, check: impl Fn(&Cx) -> bool + Send + 'static) -> String {
        let view = HoistView::new(ThenView::new(async move {
            let cx = &cx;
            let result = check(cx);
            Ok(view! { cx => <p>(result)</p> })
        }));
        block_on(view.single()).unwrap().render(&Cx::default())
    }

    #[test]
    fn an_http_render_is_not_connected_and_requests_a_connection() {
        let html = render(Cx::default(), connected);
        assert!(html.starts_with(MARKER), "{html}");
        assert!(html.ends_with("<p>false</p>"), "{html}");
    }

    #[test]
    fn a_connected_render_is_connected_and_keeps_requesting_a_connection() {
        let html = render(Cx::default().with(ConnectedRender), connected);
        assert!(html.starts_with(MARKER), "{html}");
        assert!(html.ends_with("<p>true</p>"), "{html}");
    }

    #[test]
    fn repeated_checks_render_one_marker() {
        let html = render(Cx::default(), |cx| {
            let _ = connected(cx);
            let _ = connected(cx);
            connected(cx)
        });
        assert_eq!(html.matches(MARKER).count(), 1, "{html}");
    }

    #[test]
    fn the_untracked_check_renders_no_marker() {
        let html = render(Cx::default(), connected_untracked);
        assert!(!html.contains("::topcoat::connect"), "{html}");
        assert!(html.ends_with("<p>false</p>"), "{html}");

        let html = render(Cx::default().with(ConnectedRender), connected_untracked);
        assert!(!html.contains("::topcoat::connect"), "{html}");
        assert!(html.ends_with("<p>true</p>"), "{html}");
    }

    #[test]
    fn a_cloned_context_shares_the_render_mode() {
        let cx = Cx::default().with(ConnectedRender);
        assert!(connected_untracked(&cx.keyed(1)));
        assert!(connected_untracked(&cx.with(())));
    }

    #[test]
    fn the_tracked_check_outside_a_body_panics() {
        let cx = Cx::default();
        let panic = catch_unwind(AssertUnwindSafe(|| connected(&cx))).unwrap_err();
        let message = panic.downcast::<&str>().expect("panics with a message");
        assert!(message.contains("no view is collecting hoisted parts"));
    }

    #[test]
    fn the_untracked_check_works_outside_a_body() {
        assert!(!connected_untracked(&Cx::default()));
        assert!(connected_untracked(&Cx::default().with(ConnectedRender)));
    }
}
