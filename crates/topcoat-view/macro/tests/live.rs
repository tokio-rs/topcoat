use std::{
    future::poll_fn,
    io,
    pin::{Pin, pin},
};

use topcoat::{
    Result,
    context::Cx,
    view::{View, ViewExt, ViewFirst, ViewSwap, component, emit, live, view},
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
async fn region_emitting_once_is_not_live() {
    let cx = &Cx::default();
    let mut view = pin!(view! { cx => <main>(live! { emit! { load(fail: false) } })</main> });

    assert!(!first(&mut view).await.unwrap().live);
    assert!(next_swap(&mut view).await.unwrap().is_none());
}

#[tokio::test]
async fn region_emitting_a_view_against_its_own_cx_renders_its_content() {
    let cx = &Cx::default();
    let html = view! { cx => <main>(live! { emit! { cx => <p>"own"</p> } })</main> }
        .single()
        .await
        .unwrap()
        .render(cx);

    assert_eq!(html, "<main><p>own</p></main>");
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
    let mut view = pin!(view! {
        cx =>
        <main>
            (live! {
                emit! { <p>"first"</p> }?;
                emit! { <p>"second"</p> }
            })
        </main>
    });

    // The first content marks the region off, so the swap can find it again.
    let content = first(&mut view).await.unwrap();
    assert!(content.live);

    let swap = next_swap(&mut view).await.unwrap().unwrap();
    let region = swap.region;
    assert_eq!(
        content.content.render(cx),
        format!("<main><!--tc:{region}--><p>first</p><!--/tc:{region}--></main>")
    );
    assert_eq!(swap.replacement.render(cx), "<p>second</p>");

    // The body ran out of emissions, so the region is done.
    assert!(next_swap(&mut view).await.unwrap().is_none());
}

#[tokio::test]
async fn region_emitting_three_times_swaps_its_content_twice() {
    let cx = &Cx::default();
    let mut view = pin!(view! {
        cx =>
        <main>
            (live! {
                emit! { <p>"one"</p> }?;
                emit! { <p>"two"</p> }?;
                emit! { <p>"three"</p> }
            })
        </main>
    });

    assert!(first(&mut view).await.unwrap().live);
    let swap = next_swap(&mut view).await.unwrap().unwrap();
    assert_eq!(swap.replacement.render(cx), "<p>two</p>");
    let swap = next_swap(&mut view).await.unwrap().unwrap();
    assert_eq!(swap.replacement.render(cx), "<p>three</p>");
    assert!(next_swap(&mut view).await.unwrap().is_none());
}

#[tokio::test]
async fn first_loop_iteration_delivers_its_swap() {
    let cx = &Cx::default();
    let mut view = pin!(view! {
        cx =>
        <ul>
            for label in ["only"] {
                <li>(live! {
                    emit! { <i>(label) "1"</i> }?;
                    emit! { <i>(label) "2"</i> }
                })</li>
            }
        </ul>
    });

    assert!(first(&mut view).await.unwrap().live);
    let swap = next_swap(&mut view).await.unwrap().unwrap();
    assert_eq!(swap.replacement.render(cx), "<i>only2</i>");
    assert!(next_swap(&mut view).await.unwrap().is_none());
}

#[tokio::test]
async fn live_loop_iterations_take_turns_swapping() {
    let cx = &Cx::default();
    let mut view = pin!(view! {
        cx =>
        <ul>
            for label in ["a", "b"] {
                <li>(live! {
                    emit! { <i>(label) "1"</i> }?;
                    emit! { <i>(label) "2"</i> }?;
                    emit! { <i>(label) "3"</i> }
                })</li>
            }
        </ul>
    });

    assert!(first(&mut view).await.unwrap().live);

    let mut swaps = Vec::new();
    while let Some(swap) = next_swap(&mut view).await.unwrap() {
        swaps.push(swap.replacement.render(cx));
    }
    assert_eq!(swaps, ["<i>a2</i>", "<i>b2</i>", "<i>a3</i>", "<i>b3</i>"]);
}

#[tokio::test]
#[should_panic(expected = "used `.single()` on a View that is live")]
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
