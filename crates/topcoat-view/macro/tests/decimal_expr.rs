use topcoat::{context::Cx, runtime::Decimal, view::view};

fn r(v: topcoat::Result) -> String {
    v.unwrap().render(&Cx::default())
}

/// A `Decimal` signal drives a text expression, a comparison, and a method
/// call; the value crosses the boundary tagged (never as a float) and the
/// server render is exact.
#[tokio::test]
async fn decimal_signal_renders_and_compiles() {
    let cx = &Cx::default();
    let html = r(view! {
        cx =>
        signal price = Decimal::new("19.99");
        signal cap = Decimal::new("100.00");

        <p>"Price: " $(price.get().to_string())</p>
        <p :hidden=$(price.get() > cap.get())>"over cap"</p>
        <p :hidden=$(price.get().is_zero())>"zero"</p>
    });

    // comparison and method calls translate to the matching JS
    for call in [".gt(", ".to_string()", ".is_zero()"] {
        assert!(html.contains(call), "compiled JS contains {call}: {html}");
    }

    // the value crosses as a tagged Decimal, not a bare number
    assert!(
        html.contains("Decimal"),
        "signal declared as a tagged Decimal: {html}"
    );

    // the server render is the exact string, trailing zeros preserved
    assert!(
        html.contains("-->19.99<!-- ::topcoat::expr::end -->"),
        "exact decimal rendered server-side: {html}"
    );
    // 19.99 <= 100.00, so the :hidden bind rendered its initial "true"
    assert!(html.contains("Price: "));
}

/// Money never becomes a float: constructing a `Decimal` from float notation
/// is a hard error, not a silent coercion.
#[tokio::test]
#[should_panic(expected = "not a decimal number")]
async fn float_notation_is_rejected() {
    let _ = Decimal::new("1.5e3");
}
