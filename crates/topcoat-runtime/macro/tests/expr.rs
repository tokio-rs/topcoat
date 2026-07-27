//! The signal write methods, checked through the macro that compiles them.
//!
//! Each case renders a view whose handler calls one of the methods and asserts
//! that the name reaches the generated JavaScript unchanged. The name is the
//! interface to the browser runtime: nothing maps it on the way out, so a
//! rename on one side alone would fail only in the browser, at click time.

use topcoat::{context::Cx, view::view};

#[tokio::test]
async fn toggle_reaches_the_generated_javascript() {
    let cx = &Cx::default();
    let html = view! {
        cx =>
        signal open = false;

        <button @click=$(|_e| open.toggle())>"x"</button>
    }
    .unwrap()
    .render(cx);

    assert!(html.contains(".toggle()"), "{html}");
}

#[tokio::test]
async fn increment_and_decrement_reach_the_generated_javascript() {
    let cx = &Cx::default();
    let html = view! {
        cx =>
        signal count = 0.0;

        <button @click=$(|_e| count.increment())>"+"</button>
        <button @click=$(|_e| count.decrement())>"-"</button>
    }
    .unwrap()
    .render(cx);

    assert!(html.contains(".increment()"), "{html}");
    assert!(html.contains(".decrement()"), "{html}");
}

#[tokio::test]
async fn push_str_reaches_the_generated_javascript_with_its_argument() {
    let cx = &Cx::default();
    let html = view! {
        cx =>
        signal name = String::new();

        <button @click=$(|_e| name.push_str("!"))>"x"</button>
    }
    .unwrap()
    .render(cx);

    assert!(html.contains(".push_str("), "{html}");
}
