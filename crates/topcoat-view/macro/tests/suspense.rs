use std::{
    future::poll_fn,
    io,
    pin::{Pin, pin},
};

use tokio::sync::oneshot;
use topcoat::{
    Result,
    context::Cx,
    view::{View, ViewFirst, ViewSwap, component, emit, live, suspense, view},
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
            (view! { <b>"prefix"</b> })
            slow(rx: rx)
        )
    });

    let content = first(&mut view).await.unwrap();
    assert!(content.live);
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
    assert!(!content.live);
    assert_eq!(content.content.render(cx), "<p>content</p>");
    assert!(next_swap(&mut view).await.unwrap().is_none());
}

#[tokio::test]
async fn suspense_forwards_a_ready_childs_swaps() {
    let cx = &Cx::default();
    let mut view = pin!(view! {
        cx =>
        suspense(
            fallback: view! { <p>"loading"</p> },
            (live! {
                emit! { <i>"first"</i> }?;
                emit! { <i>"updated"</i> }
            })
        )
    });

    let content = first(&mut view).await.unwrap();
    assert!(content.live);
    let html = content.content.render(cx);
    let swap = next_swap(&mut view).await.unwrap().unwrap();
    assert_eq!(
        html,
        format!(
            "<!--topcoat::region::start({})--><i>first</i><!--topcoat::region::end({})-->",
            swap.region, swap.region,
        ),
    );
    assert_eq!(swap.replacement.render(cx), "<i>updated</i>");
    assert!(next_swap(&mut view).await.unwrap().is_none());
}

#[tokio::test]
async fn suspense_forwards_child_swaps_after_replacing_the_fallback() {
    let cx = &Cx::default();
    let (tx, rx) = oneshot::channel::<()>();
    let mut view = pin!(view! {
        cx =>
        suspense(
            fallback: view! { <p>"loading"</p> },
            (live! {
                rx.await.unwrap();
                emit! { <i>"first"</i> }?;
                emit! { <i>"updated"</i> }
            })
        )
    });

    let content = first(&mut view).await.unwrap();
    assert!(content.live);
    let html = content.content.render(cx);
    tx.send(()).unwrap();

    let replacement = next_swap(&mut view).await.unwrap().unwrap();
    assert_eq!(
        html,
        format!(
            "<!--topcoat::region::start({})--><p>loading</p><!--topcoat::region::end({})-->",
            replacement.region, replacement.region,
        ),
    );
    let child_html = replacement.replacement.render(cx);
    let update = next_swap(&mut view).await.unwrap().unwrap();
    assert_ne!(replacement.region, update.region);
    assert_eq!(
        child_html,
        format!(
            "<!--topcoat::region::start({})--><i>first</i><!--topcoat::region::end({})-->",
            update.region, update.region,
        ),
    );
    assert_eq!(update.replacement.render(cx), "<i>updated</i>");
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
