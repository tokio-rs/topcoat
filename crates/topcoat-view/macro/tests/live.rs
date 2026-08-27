use std::{future::poll_fn, io, pin::Pin};

use futures_core::Stream;
use topcoat::{
    Result,
    context::Cx,
    view::{Swap, Swaps, View, ViewExt, component, emit, live, view},
};

#[component]
async fn load(fail: bool) -> Result<impl View> {
    if fail {
        return Err(io::Error::other("boom").into());
    }
    Ok(view! { <p>"loaded"</p> })
}

async fn next<V: View>(swaps: &mut Swaps<V>) -> Option<Result<Swap>> {
    poll_fn(|cx| Pin::new(&mut *swaps).poll_next(cx)).await
}

#[tokio::test]
async fn region_emitting_once_renders_as_plain_content() {
    let cx = &Cx::default();
    let html = view! { cx => <main>(live! { emit! { load(fail: false) } })</main> }
        .single()
        .await
        .unwrap()
        .render(cx);

    assert_eq!(html, "<main><p>loaded</p></main>");
}

#[tokio::test]
async fn region_remapping_a_failed_emission_renders_as_plain_content() {
    let cx = &Cx::default();
    let html = view! {
        cx =>
        <main>
            (live! {
                if let Err(error) = emit! { load(fail: true) } {
                    emit! { <p class="error">(error.to_string())</p> }?;
                }
                Ok(())
            })
        </main>
    }
    .single()
    .await
    .unwrap()
    .render(cx);

    assert_eq!(html, r#"<main><p class="error">boom</p></main>"#);
}

#[tokio::test]
async fn region_failing_before_its_content_fails_the_view() {
    let cx = &Cx::default();
    let error = view! { cx => <main>(live! { emit! { load(fail: true) } })</main> }
        .single()
        .await
        .unwrap_err();

    assert_eq!(error.to_string(), "boom");
}

#[tokio::test]
async fn region_failing_after_an_emission_fails_the_view() {
    let cx = &Cx::default();
    let error = view! {
        cx =>
        <main>
            (live! {
                emit! { <p>"first"</p> }?;
                Err(io::Error::other("late").into())
            })
        </main>
    }
    .single()
    .await
    .unwrap_err();

    assert_eq!(error.to_string(), "late");
}

#[tokio::test]
async fn region_emitting_twice_swaps_its_content() {
    let cx = &Cx::default();
    let (content, mut swaps) = view! {
        cx =>
        <main>
            (live! {
                emit! { <p>"first"</p> }?;
                emit! { <p>"second"</p> }
            })
        </main>
    }
    .live()
    .await
    .unwrap();
    assert!(swaps.is_live());

    let swap = next(&mut swaps).await.unwrap().unwrap();
    let region = swap.region;
    assert_eq!(
        content.render(cx),
        format!("<main><!--tc:{region}--><p>first</p><!--/tc:{region}--></main>")
    );
    assert_eq!(swap.replacement.render(cx), "<p>second</p>");
    assert!(!swaps.is_live());
    assert!(next(&mut swaps).await.is_none());
}

#[tokio::test]
#[should_panic(expected = "`single` called on a live view")]
async fn single_panics_on_a_region_that_may_update() {
    let cx = &Cx::default();
    let _ = view! {
        cx =>
        <main>
            (live! {
                emit! { <p>"first"</p> }?;
                emit! { <p>"second"</p> }
            })
        </main>
    }
    .single()
    .await;
}
