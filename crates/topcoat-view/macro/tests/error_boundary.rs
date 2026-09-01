use std::{
    future::poll_fn,
    io,
    pin::{Pin, pin},
};

use tokio::sync::oneshot;
use topcoat::{
    Result,
    context::Cx,
    view::{View, ViewExt, ViewFirst, ViewSwap, component, emit, error_boundary, live, view},
};

#[component]
async fn load(fail: bool) -> Result<impl View> {
    if fail {
        return Err(io::Error::other("boom").into());
    }
    Ok(view! { <p>"loaded"</p> })
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
async fn error_boundary_renders_child_content_as_plain_content() {
    let cx = &Cx::default();
    let html = view! {
        cx =>
        error_boundary(fallback: |_| Ok(view! { <p>"failed"</p> }), <p>"content"</p>)
    }
    .single()
    .await
    .unwrap()
    .render(cx);

    assert_eq!(html, "<p>content</p>");
}

#[tokio::test]
async fn error_boundary_shows_the_fallback_for_a_failed_child() {
    let cx = &Cx::default();
    let html = view! {
        cx =>
        error_boundary(
            fallback: |error| Ok(view! { <p class="error">(error.to_string())</p> }),
            load(fail: true)
        )
    }
    .single()
    .await
    .unwrap()
    .render(cx);

    assert_eq!(html, r#"<p class="error">boom</p>"#);
}

#[tokio::test]
async fn error_boundary_rethrows_an_error_the_fallback_returns() {
    let cx = &Cx::default();
    let error = view! {
        cx =>
        error_boundary(
            fallback: |error| {
                if error.to_string() == "boom" {
                    return Err(error);
                }
                Ok(view! { <p>"handled"</p> })
            },
            load(fail: true)
        )
    }
    .single()
    .await
    .unwrap_err();

    assert_eq!(error.to_string(), "boom");
}

#[tokio::test]
async fn error_boundary_replaces_streamed_content_on_a_late_error() {
    let cx = &Cx::default();
    let (tx, rx) = oneshot::channel::<()>();
    let mut view = pin!(view! {
        cx =>
        error_boundary(
            fallback: |error| Ok(view! { <p class="error">(error.to_string())</p> }),
            (live! {
                emit! { <p>"partial"</p> }?;
                rx.await.ok();
                Err(io::Error::other("late").into())
            })
        )
    });

    let content = first(&mut view).await.unwrap();
    assert!(content.live);
    assert!(content.content.render(cx).contains("<p>partial</p>"));

    let _ = tx.send(());
    let swap = next_swap(&mut view).await.unwrap().unwrap();
    assert_eq!(swap.replacement.render(cx), r#"<p class="error">late</p>"#);
    assert!(next_swap(&mut view).await.unwrap().is_none());
}
