//! Expression rendering, value conversions, and compiled signal handlers.
//!
//! Each case renders a component whose handler calls one of the methods and
//! asserts that the name reaches the generated JavaScript unchanged. The name
//! is the interface to the browser runtime: nothing maps it on the way out,
//! so a rename on one side alone would fail only in the browser, at click
//! time.

use topcoat::{
    Result,
    context::Cx,
    runtime::{Event, Expr, expr, procedure, signal},
    view::{View, ViewExt, attributes, component, view},
};

#[component]
async fn nested_expressions(cx: &Cx) -> Result<impl View> {
    let selected = signal(cx, || true);
    let active = expr!(selected.get());
    let label = expr!(if active { "true" } else { "false" });
    let repeated = expr!(label.to_owned());
    assert!(!label.is_static());
    assert!(!repeated.is_static());

    for active in [Expr::from(false), expr!(true)] {
        let label = expr!(if active { "true" } else { "false" });
        assert!(label.is_static());
    }

    Ok(view! {
        <button :aria-selected=(label) :tabindex=$(if active { 0i32 } else { -1i32 })>
            (repeated)
        </button>
    })
}

#[tokio::test]
async fn nested_expressions_preserve_static_and_dynamic_bindings() {
    let cx = &Cx::default();
    let html = view! { cx => nested_expressions() }
        .single()
        .await
        .unwrap()
        .render(cx);
    assert!(html.contains("aria-selected=\"true\""), "{html}");
    assert!(html.contains("tabindex=\"0\""), "{html}");
    assert!(html.contains("data-topcoat-bind:aria-selected"), "{html}");
    assert!(html.contains("data-topcoat-bind:tabindex"), "{html}");

    assert!(!html.contains("::topcoat::dep("), "{html}");
}

#[test]
fn capturing_an_expression_reuses_its_server_value() {
    let evaluations = std::cell::Cell::new(0);
    let value = expr!(raw!("cx.hydrate(7)", {
        evaluations.set(evaluations.get() + 1);
        7.0
    }));
    let doubled = expr!(value + value);
    assert_eq!(doubled.into_evaluated_and_js().0.to_bits(), 14.0f64.to_bits());
    assert_eq!(evaluations.get(), 1);
}

#[tokio::test]
async fn trailing_comma_in_an_inline_expression_keeps_both_attributes() {
    let cx = &Cx::default();
    let dark = false;
    let html = view! {
        cx =>
        let label = expr!(
            if dark { "Switch to light theme" } else { "Switch to dark theme" },
        );

        <button :title=(label.clone()) :aria-label=(label)>"Theme"</button>
    }
    .single()
    .await
    .unwrap()
    .render(cx);

    assert!(html.contains("title=\"Switch to dark theme\""), "{html}");
    assert!(
        html.contains("aria-label=\"Switch to dark theme\""),
        "{html}"
    );
    assert!(!html.contains("data-topcoat-bind:"), "{html}");
}

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

#[test]
fn bool_then_avoids_javascript_thenable_assimilation() {
    let (_, js) = expr!(true.then(|| "yes").unwrap()).into_evaluated_and_js();
    let source = js.to_source();
    assert!(source.contains(".then_("), "{source}");
    assert!(!source.contains(".then("), "{source}");
}

#[component]
async fn counter_buttons(cx: &Cx) -> Result<impl View> {
    let count = signal(cx, || 0i32);
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
    let html = view! { cx => captured_text(text: spicy) }
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

#[component]
async fn captured_text(cx: &Cx, text: String) -> Result<impl View> {
    let shown = signal(cx, || true);
    Ok(view! { <p>$(if shown.get() { text.to_owned() } else { "".to_owned() })</p> })
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

#[component]
async fn panel(#[into] open: Expr<bool>) -> Result<impl View> {
    Ok(view! { <dialog :open=(open)></dialog> })
}

#[tokio::test]
async fn value_conversion_renders_props_as_ordinary_attributes() {
    let cx = &Cx::default();
    let html = view! {
        cx =>
        panel(open: true)
        panel(open: false)
    }
    .single()
    .await
    .unwrap()
    .render(cx);

    assert_eq!(html, "<dialog open=\"\"></dialog><dialog></dialog>");
}

#[component]
async fn interactive_panel(cx: &Cx) -> Result<impl View> {
    let open = signal(cx, || false);
    Ok(view! { panel(open: $(open.get())) })
}

#[tokio::test]
async fn expression_conversion_preserves_runtime_prop_bindings() {
    let cx = &Cx::default();
    let html = view! { cx => interactive_panel() }
        .single()
        .await
        .unwrap()
        .render(cx);

    assert!(html.contains("data-topcoat-bind:open="), "{html}");
    assert!(html.contains(".get()"), "{html}");
}

#[tokio::test]
async fn value_conversion_renders_nodes_without_markers_and_escapes_html() {
    let cx = &Cx::default();
    let text = Expr::from("<b>&");
    let html = view! { cx => <p>(text)</p> }
        .single()
        .await
        .unwrap()
        .render(cx);

    assert_eq!(html, "<p>&lt;b&gt;&amp;</p>");
}

#[tokio::test]
async fn constants_and_captured_values_render_without_bindings() {
    let cx = &Cx::default();
    assert!(expr!(true).is_static());
    let captured = String::from("<&");
    assert!(expr!(captured.to_owned()).is_static());
    let html = view! {
        cx =>
        <p :title=$(captured.to_owned())>$(captured.to_owned())</p>
    }
    .single()
    .await
    .unwrap()
    .render(cx);

    assert_eq!(html, "<p title=\"<&amp;\">&lt;&amp;</p>");
}

#[tokio::test]
async fn forwarded_value_conversions_replace_dynamic_bindings() {
    let cx = &Cx::default();
    let html = view! { cx => replaced_dynamic_bindings() }
        .single()
        .await
        .unwrap()
        .render(cx);

    assert!(html.contains("value=\"&quot;<&amp;\""), "{html}");
    assert!(html.contains(" required"), "{html}");
    assert!(!html.contains("disabled"), "{html}");
    assert!(!html.contains("data-topcoat-bind:"), "{html}");
}

#[component]
async fn replaced_dynamic_bindings(cx: &Cx) -> Result<impl View> {
    let value = signal(cx, || String::from("old"));
    let disabled = signal(cx, || true);
    let previous = attributes! { :value=$(value.get()) :disabled=$(disabled.get()) };
    let attrs = attributes! {
        (previous)
        :value=(Expr::from("\"<&"))
        :disabled=(Expr::from(false))
        :required=(Expr::from(true))
    };
    Ok(view! { <input (attrs)> })
}

#[tokio::test]
async fn forwarded_dynamic_bindings_replace_value_conversions() {
    let cx = &Cx::default();
    let html = view! { cx => replaced_static_binding() }
        .single()
        .await
        .unwrap()
        .render(cx);

    assert!(html.contains("value=\"new\""), "{html}");
    assert!(html.contains("data-topcoat-bind:value="), "{html}");
}

#[component]
async fn replaced_static_binding(cx: &Cx) -> Result<impl View> {
    let value = signal(cx, || String::from("new"));
    let previous = attributes! { :value=(Expr::from("old")) };
    let attrs = attributes! { (previous) :value=$(value.get()) };
    Ok(view! { <input (attrs)> })
}

#[component]
async fn observed_expressions(cx: &Cx) -> Result<impl View> {
    let count = signal(cx, || 1usize);
    let text = signal(cx, || String::from("hello"));
    assert!(!expr!(count.get()).is_static());
    assert!(!expr!(text.read().len()).is_static());
    assert!(expr!(count).is_static());
    assert!(expr!(if false { count.get() } else { 0 }).is_static());
    assert!(!expr!(if count.get() > 0 { "yes" } else { "no" }).is_static());

    let fallback = expr!({
        let current = count;
        raw!("${current}.get()", current.get())
    });
    assert!(!fallback.is_static());
    let borrowed_fallback = expr!({
        let current = text;
        raw!("${current}.get().len()", current.read().len())
    });
    assert!(!borrowed_fallback.is_static());
    assert!(expr!(raw!("cx.hydrate(true)", true)).is_static());

    let handler = expr!(|_e: Event| count.increment());
    assert!(handler.is_static());
    let async_handler = expr!(async |_e: Event| {
        test_procedure("hello".to_owned()).await;
    });
    assert!(async_handler.is_static());

    Ok(view! {
        <button @click=(handler) @input=(async_handler)>"Run"</button>
        <p>(fallback)</p>
        <input :value=(borrowed_fallback)>
    })
}

#[tokio::test]
async fn signal_reads_and_raw_fallbacks_keep_bindings_but_not_server_dependencies() {
    let cx = &Cx::default();
    let html = view! { cx => observed_expressions() }
        .single()
        .await
        .unwrap()
        .render(cx);

    assert!(html.contains("::topcoat::expr::start("), "{html}");
    assert!(html.contains("data-topcoat-bind:value="), "{html}");
    assert!(html.contains("data-topcoat-on:click="), "{html}");
    assert!(html.contains("data-topcoat-on:input="), "{html}");
    assert!(!html.contains("::topcoat::dep("), "{html}");
}
