//! The signal write methods, checked through the macro that compiles them.
//!
//! Each case renders a view whose handler calls one of the methods and asserts
//! that the name reaches the generated JavaScript unchanged. The name is the
//! interface to the browser runtime: nothing maps it on the way out, so a
//! rename on one side alone would fail only in the browser, at click time.

use topcoat::{context::Cx, runtime::procedure, view::view};

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

#[procedure]
async fn test_procedure(v: String) -> Result<String, std::convert::Infallible> {
    Ok(v)
}

#[tokio::test]
async fn procedure_call_inside_if_is_an_async_func() {
    let cx = &Cx::default();
    let html = view! {
        cx =>
        <button
            @click=$(async |_e| {
                if true {
                    let _v = test_procedure("hello test".to_owned()).await;
                }
            })
        >
            "Test"
        </button>
    }
    .unwrap()
    .render(cx);

    assert!(
        html.contains("(await (async ()"),
        "if block containing an await must be an async anonymous function: {html}"
    );
}

#[tokio::test]
async fn procedure_call_inside_block_is_an_async_func() {
    let cx = &Cx::default();
    let html = view! {
        cx =>
        <button
            @click=$(async |_e| {
                {
                    let _v = test_procedure("hello test".to_owned()).await;
                }
            })
        >
            "Test"
        </button>
    }
    .unwrap()
    .render(cx);

    assert!(
        html.contains("(await (async ()"),
        "block containing an await must be an async anonymous function: {html}"
    );
}

/// `push_str` takes anything that dereferences to a string, so it accepts the
/// owned surrogate an event field yields. `Event::target.value` is a `String`,
/// so this is the call the method mostly exists for; the test above passes a
/// literal, which is borrowed and so never exercised the owned case.
#[tokio::test]
async fn push_str_accepts_the_owned_string_from_an_event() {
    let cx = &Cx::default();
    let html = view! {
        cx =>
        signal message = String::new();

        <input
            @input=$(|e: topcoat::runtime::Event| { message.push_str(e.target.value) })
        >
    }
    .unwrap()
    .render(cx);

    assert!(html.contains(".push_str("), "{html}");
}
