use std::sync::atomic::{AtomicUsize, Ordering};

use topcoat::{
    Result,
    context::Cx,
    view::{View, ViewExt, component, view},
};

#[component]
async fn item(n: usize) -> Result<impl View> {
    Ok(view! { <i>(n)</i> })
}

#[tokio::test]
async fn hundreds_of_dynamic_positions_in_one_template_compile_and_render_in_order() {
    let cx = &Cx::default();
    let counter = AtomicUsize::new(0);
    let next = || counter.fetch_add(1, Ordering::Relaxed);

    // 128 interpolations and 128 component calls in a single template:
    // past the depth the compiler allows for a type that nests once per
    // dynamic position.
    let html = view! {
        cx =>
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
        <p>(next())</p>
        item(n: next())
    }
    .boxed()
    .single()
    .await
    .unwrap()
    .render(cx);

    // Expressions run in source order, so each position shows its index.
    let expected = (0..256)
        .map(|n| {
            if n % 2 == 0 {
                format!("<p>{n}</p>")
            } else {
                format!("<i>{n}</i>")
            }
        })
        .collect::<String>();
    assert_eq!(html, expected);
}
