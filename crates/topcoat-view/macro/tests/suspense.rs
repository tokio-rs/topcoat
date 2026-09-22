use std::{
    future::poll_fn,
    io,
    pin::{Pin, pin},
    task::{Context, Poll},
};

use tokio::sync::oneshot;
use topcoat::{
    Result,
    context::Cx,
    core::identity::{Identity, SiteKey},
    view::{
        RegionId, View, ViewExt, ViewFirst, ViewSwap, component,
        internal::{SuspenseView, ThenView},
        suspense, view,
    },
};

/// A child whose first content must not be requested on a reconnect.
struct SwapOnly {
    swap: Option<Result<ViewSwap>>,
    pending: bool,
}

impl View for SwapOnly {
    fn poll_first(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<ViewFirst>> {
        panic!("child first content was requested");
    }

    fn poll_swap(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Option<ViewSwap>>> {
        if self.pending {
            self.pending = false;
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        Poll::Ready(self.swap.take().transpose())
    }
}

/// Renders the label the channel delivers, or fails with its error.
#[component]
async fn slow(rx: oneshot::Receiver<Result<&'static str>>) -> Result<impl View> {
    let label = rx.await.unwrap()?;
    Ok(view! { <i>(label)</i> })
}

/// Polls `view` for its first content.
async fn first<V: View>(view: &mut Pin<&mut V>) -> Result<ViewFirst> {
    poll_fn(|cx| view.as_mut().poll_first(cx)).await
}

/// Polls `view` for its next swap.
async fn next_swap<V: View>(view: &mut Pin<&mut V>) -> Result<Option<ViewSwap>> {
    poll_fn(|cx| view.as_mut().poll_swap(cx)).await
}

#[tokio::test]
async fn suspense_shows_the_fallback_until_the_child_is_ready() {
    let cx = &Cx::default();
    let (tx, rx) = oneshot::channel();
    let mut view = pin!(view! {
        cx =>
        suspense(
            fallback: view! { <p>"loading"</p> },
            (view! { <b>"prefix"</b> })
            slow(rx: rx)
        )
    });

    let content = first(&mut view).await.unwrap();
    assert!(content.streaming);
    let html = content.content.render(cx);
    assert!(html.contains("<!--topcoat::region::start("), "{html}");
    assert!(html.contains("<p>loading</p>"), "{html}");

    tx.send(Ok("done")).unwrap();
    let swap = next_swap(&mut view).await.unwrap().unwrap();
    assert_eq!(swap.replacement.render(cx), "<b>prefix</b><i>done</i>");
    assert!(next_swap(&mut view).await.unwrap().is_none());
}

#[tokio::test]
async fn suspense_renders_a_ready_child_in_place() {
    let cx = &Cx::default();
    let mut view = pin!(view! {
        cx =>
        suspense(
            fallback: view! { <p>"loading"</p> },
            <p>"content"</p>
        )
    });

    // The child is ready on the first poll, so no region is created and the
    // fallback never shows.
    let content = first(&mut view).await.unwrap();
    assert!(content.is_settled());
    assert_eq!(content.content.render(cx), "<p>content</p>");
    assert!(next_swap(&mut view).await.unwrap().is_none());
}

#[tokio::test]
async fn suspense_forwards_child_swaps_without_rendering_first_content() {
    let cx = &Cx::default();
    let region = RegionId::new(Identity::ROOT, SiteKey::new(file!(), line!(), column!(), 0));
    let child_region = RegionId::new(Identity::ROOT, SiteKey::new(file!(), line!(), column!(), 0));
    let replacement = view! { cx => <i>"update"</i> }.single().await.unwrap();
    let fallback =
        ThenView::new(async { Err::<(), _>(io::Error::other("fallback was polled").into()) });
    let mut view = pin!(SuspenseView::new(
        region,
        fallback,
        SwapOnly {
            swap: Some(Ok(ViewSwap {
                region: child_region,
                replacement,
            })),
            pending: true,
        },
    ));

    let swap = next_swap(&mut view).await.unwrap().unwrap();
    assert_ne!(region, child_region);
    assert_eq!(swap.region, child_region);
    assert_eq!(swap.replacement.render(cx), "<i>update</i>");
    assert!(next_swap(&mut view).await.unwrap().is_none());
}

#[tokio::test]
async fn suspense_forwards_a_child_swap_error() {
    let region = RegionId::new(Identity::ROOT, SiteKey::new(file!(), line!(), column!(), 0));
    let mut view = pin!(SuspenseView::new(
        region,
        (),
        SwapOnly {
            swap: Some(Err(io::Error::other("swap failed").into())),
            pending: false,
        },
    ));

    let error = next_swap(&mut view).await.unwrap_err();
    assert_eq!(error.to_string(), "swap failed");
}

#[tokio::test]
async fn suspense_propagates_a_child_error() {
    let cx = &Cx::default();
    let (tx, rx) = oneshot::channel();
    let mut view = pin!(view! {
        cx =>
        suspense(
            fallback: view! { <p>"loading"</p> },
            slow(rx: rx)
        )
    });

    assert!(first(&mut view).await.unwrap().streaming);

    let _ = tx.send(Err(io::Error::other("boom").into()));
    let error = next_swap(&mut view).await.unwrap_err();
    assert_eq!(error.to_string(), "boom");
}
