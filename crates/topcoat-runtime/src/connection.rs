use std::any::TypeId;

use topcoat_core::context::{Cx, try_request_context};
use topcoat_view::{HoistKey, hoist_once};

/// Identifies a render requested through a browser connection.
///
/// Add this to the request context when rendering over a connection.
/// Leave it out for HTTP renders.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct ConnectedRender;

/// Returns whether this render runs over a browser connection and marks
/// the content as needing one.
///
/// During an HTTP page render, this returns `false`. Once the response
/// finishes, the browser opens a connection and renders the page again.
/// That render returns `true`.
///
/// The connection request belongs to the enclosing page or shard. Layouts
/// and components belong to the page that renders them. Shards record the
/// request, but opening connections for shards is not supported yet.
///
/// Calling this requests a connection even if you only use the result to
/// choose some text. Use [`connected_untracked`] to check without requesting
/// one. Both functions describe this render, regardless of connections
/// elsewhere in the document.
///
/// # Panics
///
/// Panics when called outside a rendering scope. Spawned tasks need their
/// own rendering scope.
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

/// Returns whether this render runs over a browser connection without
/// asking the browser to open one.
///
/// Use this to change what a component displays based on the connection.
/// It works outside rendering scopes too and adds nothing to the output.
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

    /// Polls a future until it completes, without waiting between polls.
    fn block_on<F: Future>(future: F) -> F::Output {
        let mut future = pin!(future);
        let mut cx = Context::from_waker(Waker::noop());
        loop {
            if let Poll::Ready(output) = future.as_mut().poll(&mut cx) {
                return output;
            }
        }
    }

    /// Renders the result of `check` using the supplied context.
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
