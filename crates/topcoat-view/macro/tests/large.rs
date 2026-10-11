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

/// Expands to a `view!` that repeats `nodes` 16 times inside a `<div>`.
macro_rules! view_x16 {
    ($cx:ident => $($node:tt)*) => {
        view! {
            $cx =>
            <div>
                $($node)* $($node)* $($node)* $($node)*
                $($node)* $($node)* $($node)* $($node)*
                $($node)* $($node)* $($node)* $($node)*
                $($node)* $($node)* $($node)* $($node)*
            </div>
        }
    };
}

#[tokio::test]
async fn hundreds_of_dynamic_positions_in_one_template_compile_and_render_in_order() {
    let cx = &Cx::default();
    let counter = AtomicUsize::new(0);
    let next = || counter.fetch_add(1, Ordering::Relaxed);

    // 256 interpolations and 256 component calls in a single template:
    // well past the depth the compiler allows for a type that nests once
    // per dynamic position.
    let html = view_x16! {
        cx =>
        <p>(next())</p> item(n: next()) <p>(next())</p> item(n: next())
        <p>(next())</p> item(n: next()) <p>(next())</p> item(n: next())
        <p>(next())</p> item(n: next()) <p>(next())</p> item(n: next())
        <p>(next())</p> item(n: next()) <p>(next())</p> item(n: next())
        <p>(next())</p> item(n: next()) <p>(next())</p> item(n: next())
        <p>(next())</p> item(n: next()) <p>(next())</p> item(n: next())
        <p>(next())</p> item(n: next()) <p>(next())</p> item(n: next())
        <p>(next())</p> item(n: next()) <p>(next())</p> item(n: next())
    }
    .single()
    .await
    .unwrap()
    .render(cx);

    let expected = (0..512)
        .map(|n| {
            if n % 2 == 0 {
                format!("<p>{n}</p>")
            } else {
                format!("<i>{n}</i>")
            }
        })
        .collect::<String>();
    assert_eq!(html, format!("<div>{expected}</div>"));
}
