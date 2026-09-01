use std::{
    future::poll_fn,
    io,
    pin::{Pin, pin},
};

use tokio::sync::oneshot;
use topcoat::{
    Result,
    context::Cx,
    view::{View, ViewFirst, ViewSwap, component, suspense, view},
};

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
            slow(rx: rx)
        )
    });

    let content = first(&mut view).await.unwrap();
    assert!(content.live);
    assert!(content.content.render(cx).contains("<p>loading</p>"));

    tx.send(Ok("done")).unwrap();
    let swap = next_swap(&mut view).await.unwrap().unwrap();
    assert_eq!(swap.replacement.render(cx), "<i>done</i>");
    assert!(next_swap(&mut view).await.unwrap().is_none());
}

#[tokio::test]
async fn suspense_swaps_in_an_immediate_child() {
    let cx = &Cx::default();
    let mut view = pin!(view! {
        cx =>
        suspense(
            fallback: view! { <p>"loading"</p> },
            <p>"content"</p>
        )
    });

    assert!(first(&mut view).await.unwrap().live);
    let swap = next_swap(&mut view).await.unwrap().unwrap();
    assert_eq!(swap.replacement.render(cx), "<p>content</p>");
    assert!(next_swap(&mut view).await.unwrap().is_none());
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

    assert!(first(&mut view).await.unwrap().live);

    let _ = tx.send(Err(io::Error::other("boom").into()));
    let error = next_swap(&mut view).await.unwrap_err();
    assert_eq!(error.to_string(), "boom");
}
