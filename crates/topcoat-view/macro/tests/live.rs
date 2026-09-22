use std::{
    future::poll_fn,
    io,
    pin::{Pin, pin},
};

use topcoat::{
    Result,
    context::Cx,
    view::{View, ViewExt, ViewFirst, ViewPass, ViewSwap, component, emit, live, view},
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

    assert!(!first(&mut view).await.unwrap().streaming);
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
                match emit! { load(fail: true) } {
                    Err(error) => emit! { <p class="error">(error.to_string())</p> },
                    emitted => emitted,
                }
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
async fn an_explicit_context_is_owned_by_the_live_body() {
    let region = {
        let cx = Cx::default();
        live! { cx => emit! { load(fail: false) } }
    };
    let cx = &Cx::default();
    let html = view! { cx => (region) }.single().await.unwrap().render(cx);
    assert_eq!(html, "<p>loaded</p>");
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
async fn region_hoisting_sync_control_flow_renders() {
    let cx = &Cx::default();
    let html = view! {
        cx =>
        <main>
            (live! {
                emit! {
                    <ul>
                        for x in ["a", "b"] {
                            <li data-x=(x)>"item"</li>
                        }
                    </ul>
                }
            })
        </main>
    }
    .single()
    .await
    .unwrap()
    .render(cx);

    assert_eq!(
        html,
        r#"<main><ul><li data-x="a">item</li><li data-x="b">item</li></ul></main>"#
    );
}

#[tokio::test]
async fn swapped_emission_hoisting_sync_control_flow_renders() {
    let cx = &Cx::default();
    let mut view = pin!(view! {
        cx =>
        <main>
            (live! {
                emit! { <p>"first"</p> }?;
                emit! {
                    <ul>
                        for x in ["a", "b"] {
                            <li data-x=(x)>"item"</li>
                        }
                    </ul>
                }
            })
        </main>
    });

    assert!(first(&mut view).await.unwrap().streaming);
    let swap = next_swap(&mut view).await.unwrap().unwrap();
    assert_eq!(
        swap.replacement.render(cx),
        r#"<ul><li data-x="a">item</li><li data-x="b">item</li></ul>"#
    );
    assert!(next_swap(&mut view).await.unwrap().is_none());
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
    assert!(content.streaming);

    let swap = next_swap(&mut view).await.unwrap().unwrap();
    let region = swap.region;
    assert_eq!(
        content.content.render(cx),
        format!(
            "<main><!--topcoat::region::start({region})--><p>first</p>\
             <!--topcoat::region::end({region})--></main>"
        )
    );
    assert_eq!(swap.replacement.render(cx), "<p>second</p>");

    // The body ran out of emissions, so the region is done.
    assert!(next_swap(&mut view).await.unwrap().is_none());
}

#[tokio::test]
async fn region_ids_are_stable_across_root_views() {
    let mut previous = None;
    for _ in 0..2 {
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

        let content = first(&mut view).await.unwrap();
        assert!(content.streaming);
        let swap = next_swap(&mut view).await.unwrap().unwrap();
        let region = swap.region;
        assert_eq!(
            content.content.render(cx),
            format!(
                "<main><!--topcoat::region::start({region})--><p>first</p>\
             <!--topcoat::region::end({region})--></main>"
            )
        );
        if let Some(previous) = previous {
            assert_eq!(region, previous);
        }
        previous = Some(region);
        assert!(next_swap(&mut view).await.unwrap().is_none());
    }
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

    assert!(first(&mut view).await.unwrap().streaming);
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
            #[key(label)]
            for label in ["only"] {
                <li>(live! {
                    emit! { <i>(label) "1"</i> }?;
                    emit! { <i>(label) "2"</i> }
                })</li>
            }
        </ul>
    });

    assert!(first(&mut view).await.unwrap().streaming);
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
            #[key(label)]
            for label in ["a", "b"] {
                <li>(live! {
                    emit! { <i>(label) "1"</i> }?;
                    emit! { <i>(label) "2"</i> }?;
                    emit! { <i>(label) "3"</i> }
                })</li>
            }
        </ul>
    });

    assert!(first(&mut view).await.unwrap().streaming);

    let mut swaps = Vec::new();
    while let Some(swap) = next_swap(&mut view).await.unwrap() {
        swaps.push(swap.replacement.render(cx));
    }
    assert_eq!(swaps, ["<i>a2</i>", "<i>b2</i>", "<i>a3</i>", "<i>b3</i>"]);
}

#[tokio::test]
#[should_panic(expected = "used `.single()` on a View that changes after it went out")]
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

#[tokio::test]
async fn joined_emissions_all_reach_the_region() {
    let cx = &Cx::default();
    let mut view = pin!(view! {
        cx =>
        <main>
            (live! {
                let (a, b) = tokio::join!(
                    async { emit! { <p>"a"</p> } },
                    async { emit! { <p>"b"</p> } },
                );
                a?;
                b
            })
        </main>
    });

    // Both emissions happen in the same poll of the body. One becomes the
    // first content and the other waits its turn to swap it out.
    let content = first(&mut view).await.unwrap();
    assert!(content.streaming);
    let swap = next_swap(&mut view).await.unwrap().unwrap();
    let region = swap.region;
    assert_eq!(
        content.content.render(cx),
        format!(
            "<main><!--topcoat::region::start({region})--><p>a</p>\
             <!--topcoat::region::end({region})--></main>"
        )
    );
    assert_eq!(swap.replacement.render(cx), "<p>b</p>");
    assert!(next_swap(&mut view).await.unwrap().is_none());
}

#[tokio::test]
async fn joined_emissions_deliver_the_swaps_of_a_nested_region() {
    let cx = &Cx::default();
    let mut view = pin!(view! {
        cx =>
        <main>
            (live! {
                let (a, b) = tokio::join!(
                    async {
                        emit! {
                            (live! {
                                emit! { <i>"1"</i> }?;
                                emit! { <i>"2"</i> }
                            })
                        }
                    },
                    async { emit! { <p>"b"</p> } },
                );
                a?;
                b
            })
        </main>
    });

    let content = first(&mut view).await.unwrap();
    assert!(content.streaming);
    let html = content.content.render(cx);
    assert!(html.contains("<i>1</i>"), "{html}");

    // The nested region's swap and the sibling emission both get delivered,
    // even though they compete for the same poll.
    let mut swaps = Vec::new();
    while let Some(swap) = next_swap(&mut view).await.unwrap() {
        swaps.push(swap.replacement.render(cx));
    }
    swaps.sort();
    assert_eq!(swaps, ["<i>2</i>", "<p>b</p>"]);
}

#[tokio::test]
async fn a_region_with_both_branches_runs_the_initial_one_in_the_initial_pass() {
    let cx = &Cx::default();
    let mut view = pin!(view! { cx =>
        <main>
            (live! {
                Initial => { emit! { <p>"initial"</p> } }
                Connected => { emit! { <p>"connected"</p> } }
            })
        </main>
    });

    // The initial branch is done after one emission, but the region keeps
    // its markers for the connected phase to swap into.
    let content = first(&mut view).await.unwrap();
    assert!(!content.streaming);
    assert!(content.connecting);
    let html = content.content.render(cx);
    assert!(html.contains("<!--topcoat::region::start("), "{html}");
    assert!(html.contains("<p>initial</p>"), "{html}");
    assert!(!html.contains("connected"), "{html}");
    assert!(next_swap(&mut view).await.unwrap().is_none());
}

#[tokio::test]
async fn a_region_with_only_an_initial_branch_settles() {
    let cx = &Cx::default();
    let mut view = pin!(view! { cx =>
        <main>(live! { Initial => { emit! { <p>"initial"</p> } } })</main>
    });

    let content = first(&mut view).await.unwrap();
    assert!(!content.streaming);
    assert!(!content.connecting);
    assert_eq!(content.content.render(cx), "<main><p>initial</p></main>");
}

#[tokio::test]
async fn a_region_runs_its_connected_branch_in_the_connected_pass() {
    let cx = &Cx::default().with(ViewPass::Connected);
    let mut view = pin!(view! { cx =>
        <main>
            (live! {
                Initial => { emit! { <p>"initial"</p> } }
                Connected => { emit! { <p>"connected"</p> } }
            })
        </main>
    });

    let content = first(&mut view).await.unwrap();
    let html = content.content.render(cx);
    assert!(html.contains("<p>connected</p>"), "{html}");
    assert!(!html.contains("<p>initial</p>"), "{html}");
}

#[tokio::test]
async fn a_region_without_a_connected_branch_runs_its_initial_one_in_the_connected_pass() {
    let cx = &Cx::default().with(ViewPass::Connected);
    let mut view = pin!(view! { cx =>
        <main>(live! { Initial => { emit! { <p>"initial"</p> } } })</main>
    });

    // A region reached while rendering connected content has nothing else
    // to show, and nothing connects later.
    let content = first(&mut view).await.unwrap();
    assert!(!content.connecting);
    assert_eq!(content.content.render(cx), "<main><p>initial</p></main>");
}

#[tokio::test]
async fn branches_capture_the_same_variable() {
    let cx = &Cx::default();
    let label = String::from("shared");
    let mut view = pin!(view! { cx =>
        <main>
            (live! {
                Initial => { emit! { <p>(label.clone())</p> } }
                Connected => { emit! { <p>(label)</p> } }
            })
        </main>
    });

    let html = first(&mut view).await.unwrap().content.render(cx);
    assert!(html.contains("<p>shared</p>"), "{html}");
}

#[tokio::test]
async fn a_single_body_runs_in_the_connected_pass() {
    let cx = &Cx::default().with(ViewPass::Connected);
    let mut view = pin!(view! { cx =>
        <main>(live! { emit! { <p>"body"</p> } })</main>
    });

    let content = first(&mut view).await.unwrap();
    assert_eq!(content.content.render(cx), "<main><p>body</p></main>");
}

#[tokio::test]
async fn a_connected_body_sends_every_emission_when_polled_for_swaps_first() {
    let cx = &Cx::default().with(ViewPass::Connected);
    let mut view = pin!(view! { cx =>
        (live! {
            Initial => { Err(io::Error::other("initial body ran").into()) }
            Connected => {
                tokio::task::yield_now().await;
                emit! { <p>"current"</p> }?;
                emit! { <p>"updated"</p> }
            }
        })
    });

    let current = next_swap(&mut view).await.unwrap().unwrap();
    assert_eq!(current.replacement.render(cx), "<p>current</p>");
    let updated = next_swap(&mut view).await.unwrap().unwrap();
    assert_eq!(updated.region, current.region);
    assert_eq!(updated.replacement.render(cx), "<p>updated</p>");
    assert!(next_swap(&mut view).await.unwrap().is_none());
}

#[tokio::test]
async fn a_swap_only_pass_skips_initial_only_bodies() {
    let cx = &Cx::default().with(ViewPass::Connected);
    let mut view = pin!(view! { cx =>
        (live! {
            Err(io::Error::other("single body ran").into())
        })
        (live! {
            Initial => { Err(io::Error::other("initial branch ran").into()) }
        })
    });

    assert!(next_swap(&mut view).await.unwrap().is_none());
}

#[tokio::test]
async fn an_emitted_live_view_inherits_the_connected_pass() {
    let cx = &Cx::default().with(ViewPass::Connected);
    let mut view = pin!(view! { cx =>
        (live! {
            Initial => { Err(io::Error::other("outer initial body ran").into()) }
            Connected => {
                emit! {
                    (live! {
                        Initial => { Err(io::Error::other("inner initial body ran").into()) }
                        Connected => {
                            emit! { <i>"current"</i> }?;
                            emit! { <i>"updated"</i> }
                        }
                    })
                }
            }
        })
    });

    let outer = next_swap(&mut view).await.unwrap().unwrap();
    let html = outer.replacement.render(cx);
    let inner = next_swap(&mut view).await.unwrap().unwrap();
    assert_ne!(outer.region, inner.region);
    assert_eq!(
        html,
        format!(
            "<!--topcoat::region::start({})--><i>current</i><!--topcoat::region::end({})-->",
            inner.region, inner.region,
        ),
    );
    assert_eq!(inner.replacement.render(cx), "<i>updated</i>");
    assert!(next_swap(&mut view).await.unwrap().is_none());
}

#[tokio::test]
async fn emitted_initial_only_bodies_render_and_keep_streaming() {
    let cx = &Cx::default().with(ViewPass::Connected);
    let mut view = pin!(view! { cx =>
        (live! {
            Initial => { Err(io::Error::other("outer initial body ran").into()) }
            Connected => {
                emit! {
                    (live! {
                        emit! { <i>"single first"</i> }?;
                        emit! { <i>"single second"</i> }
                    })
                    (live! {
                        Initial => {
                            emit! { <b>"branch first"</b> }?;
                            emit! { <b>"branch second"</b> }
                        }
                    })
                }
            }
        })
    });

    let outer = next_swap(&mut view).await.unwrap().unwrap();
    let html = outer.replacement.render(cx);
    assert!(html.contains("<i>single first</i>"), "{html}");
    assert!(html.contains("<b>branch first</b>"), "{html}");

    let mut replacements = Vec::new();
    while let Some(swap) = next_swap(&mut view).await.unwrap() {
        assert_ne!(swap.region, outer.region);
        assert!(
            html.contains(&format!("<!--topcoat::region::start({})-->", swap.region)),
            "{html}",
        );
        replacements.push(swap.replacement.render(cx));
    }
    replacements.sort();
    assert_eq!(replacements, ["<b>branch second</b>", "<i>single second</i>"]);
}
