use topcoat::{
    Result,
    context::Cx,
    runtime::{Event, Surrogate, expr, record, signal},
    view::{View, ViewExt, component, view},
};

// Neither the rows nor the containing record require Clone for this lookup.
#[record]
struct Choice {
    value: String,
    rank: u32,
}

#[record]
struct Choices {
    rows: Vec<Choice>,
}

#[component]
async fn lookup(cx: &Cx) -> Result<impl View> {
    let choices = signal(cx, || Choices {
        rows: vec![
            Choice {
                value: "a".to_owned(),
                rank: 1,
            },
            Choice {
                value: "b".to_owned(),
                rank: 2,
            },
        ],
    });
    let active = signal(cx, || "b".to_owned());
    let found = signal(cx, || None::<usize>);
    let position = expr!(
        choices
            .read()
            .rows
            .iter()
            .position(|row| row.value == active.read())
    );
    assert!(!position.is_static());
    assert_eq!(position.clone().into_evaluated_and_js().0, Some(1));
    let rank = expr!(choices.read().rows.iter().position(|row| row.rank == 1u32));
    assert_eq!(rank.into_evaluated_and_js().0, Some(0));

    let callback = expr!(|| {
        let value = active.read();
        choices
            .read()
            .rows
            .iter()
            .position(|row| row.value == value)
    });
    assert!(callback.is_static());
    assert_eq!(callback.into_evaluated_and_js().0().into_real(), Some(1));

    let handler = expr!(|_event: Event| {
        let value = active.read();
        found.set(
            choices
                .read()
                .rows
                .iter()
                .position(|row| row.value == value),
        );
    });
    assert!(handler.is_static());
    Ok(view! { <button @click=(handler) :data-position=(position)>"Locate"</button> })
}

#[tokio::test]
async fn signal_borrowed_record_lookup_uses_browser_dependencies() {
    let cx = &Cx::default();
    let html = view! { cx => lookup() }.single().await.unwrap().render(cx);
    assert!(html.contains("data-position=\"1\""), "{html}");
    assert!(html.contains("data-topcoat-bind:data-position"), "{html}");
    assert!(html.contains("data-topcoat-on:click"), "{html}");
    assert!(!html.contains("::topcoat::dep("), "{html}");
}
