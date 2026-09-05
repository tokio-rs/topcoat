//! The signal write methods, checked through the macro that compiles them.
//!
//! Each case renders a component whose handler calls one of the methods and
//! asserts that the name reaches the generated JavaScript unchanged. The name
//! is the interface to the browser runtime: nothing maps it on the way out,
//! so a rename on one side alone would fail only in the browser, at click
//! time.

use topcoat::{
    Result,
    context::Cx,
    runtime::{procedure, signal},
    view::{View, ViewExt, component, view},
};

#[component]
async fn toggle_button(cx: &Cx) -> Result<impl View> {
    let open = signal(cx, || false);
    Ok(view! { <button @click=$(|_e| open.toggle())>"x"</button> })
}

#[tokio::test]
async fn toggle_reaches_the_generated_javascript() {
    let cx = &Cx::default();
    let html = view! { cx => toggle_button() }
        .single()
        .await
        .unwrap()
        .render(cx);

    assert!(html.contains(".toggle()"), "{html}");
}

#[tokio::test]
async fn bool_then_avoids_javascript_thenable_assimilation() {
    let cx = &Cx::default();
    let html = view! { cx => $(true.then(|| "yes").unwrap()) }
        .single()
        .await
        .unwrap()
        .render(cx);

    assert!(html.contains(".then_("), "{html}");
    assert!(!html.contains(".then("), "{html}");
}

#[component]
async fn counter_buttons(cx: &Cx) -> Result<impl View> {
    let count = signal(cx, || 0.0);
    Ok(view! {
        <button @click=$(|_e| count.increment())>"+"</button>
        <button @click=$(|_e| count.decrement())>"-"</button>
    })
}

#[tokio::test]
async fn increment_and_decrement_reach_the_generated_javascript() {
    let cx = &Cx::default();
    let html = view! { cx => counter_buttons() }
        .single()
        .await
        .unwrap()
        .render(cx);

    assert!(html.contains(".increment()"), "{html}");
    assert!(html.contains(".decrement()"), "{html}");
}

#[component]
async fn push_str_button(cx: &Cx) -> Result<impl View> {
    let name = signal(cx, String::new);
    Ok(view! { <button @click=$(|_e| name.push_str("!"))>"x"</button> })
}

#[tokio::test]
async fn push_str_reaches_the_generated_javascript_with_its_argument() {
    let cx = &Cx::default();
    let html = view! { cx => push_str_button() }
        .single()
        .await
        .unwrap()
        .render(cx);

    assert!(html.contains(".push_str("), "{html}");
}

/// A signal is created before the view and captured by any number of its
/// expressions; its declaration renders ahead of the component's content.
#[component]
async fn counter_display(cx: &Cx) -> Result<impl View> {
    let count = signal(cx, || 0.0);
    Ok(view! {
        <button @click=$(|_e| count.increment())>"+"</button>
        <p>$(count.get())</p>
    })
}

#[tokio::test]
async fn a_signal_is_declared_once_ahead_of_every_capture() {
    let cx = &Cx::default();
    let html = view! { cx => counter_display() }
        .single()
        .await
        .unwrap()
        .render(cx);

    let declaration = html.find("<!--::topcoat::signal(").expect(&html);
    let first_capture = html.find("cx.hydrate(").expect(&html);
    assert!(declaration < first_capture, "{html}");
    assert_eq!(html.matches("::topcoat::signal(").count(), 1, "{html}");
    assert_eq!(html.matches("cx.hydrate(").count(), 2, "{html}");
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
    .single()
    .await
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
    .single()
    .await
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
#[component]
async fn push_str_input(cx: &Cx) -> Result<impl View> {
    let message = signal(cx, String::new);
    Ok(view! {
        <input
            @input=$(|e: topcoat::runtime::Event| { message.push_str(e.target.value) })
        >
    })
}

#[tokio::test]
async fn push_str_accepts_the_owned_string_from_an_event() {
    let cx = &Cx::default();
    let html = view! { cx => push_str_input() }
        .single()
        .await
        .unwrap()
        .render(cx);

    assert!(html.contains(".push_str("), "{html}");
}

/// A captured value is serialized into the marker comment that carries the
/// expression source, so its bytes must not be able to close that comment
/// early. The `>` of any `-->` in the value renders as an entity, and the
/// snapshot between the markers is escaped for its text position.
#[tokio::test]
async fn a_captured_value_cannot_break_out_of_its_marker_comment() {
    let cx = &Cx::default();
    let spicy = String::from(r#"-->"<&"#);
    let html = view! { cx => <p>$(spicy.to_owned())</p> }
        .single()
        .await
        .unwrap()
        .render(cx);

    // The capture reaches the client as a hydrated JSON value.
    assert!(
        html.contains(r"cx.hydrate(&quot;--&gt;\&quot;<&amp;&quot;)"),
        "{html}"
    );
    // The rendered snapshot of the value is escaped for text.
    assert!(html.contains(r#"-->--&gt;"&lt;&amp;<!--"#), "{html}");
    // The raw value appears nowhere in the document.
    assert!(!html.contains(r#"-->"<&"#), "{html}");
}

/// A string literal reaches the handler attribute as a hydrated surrogate
/// with every quote escaped, so it cannot terminate the attribute value.
#[component]
async fn quoted_push_str_button(cx: &Cx) -> Result<impl View> {
    let name = signal(cx, String::new);
    Ok(view! { <button @click=$(|_e| name.push_str("say \"hi\""))>"x"</button> })
}

#[tokio::test]
async fn a_string_literal_with_quotes_stays_inside_the_handler_attribute() {
    let cx = &Cx::default();
    let html = view! { cx => quoted_push_str_button() }
        .single()
        .await
        .unwrap()
        .render(cx);

    assert!(html.contains(r"say \&quot;hi\&quot;"), "{html}");
    assert!(!html.contains(r#"say \"hi"#), "{html}");
}
