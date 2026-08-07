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
    let html = topcoat::view::scope(async {
        view! {
            cx =>
            signal open = false;

            <button @click=$(|_e| open.toggle())>"x"</button>
        }
        .unwrap()
        .render(cx)
    })
    .await;

    assert!(html.contains(".toggle()"), "{html}");
}

#[tokio::test]
async fn increment_and_decrement_reach_the_generated_javascript() {
    let cx = &Cx::default();
    let html = topcoat::view::scope(async {
        view! {
            cx =>
            signal count = 0.0;

            <button @click=$(|_e| count.increment())>"+"</button>
            <button @click=$(|_e| count.decrement())>"-"</button>
        }
        .unwrap()
        .render(cx)
    })
    .await;

    assert!(html.contains(".increment()"), "{html}");
    assert!(html.contains(".decrement()"), "{html}");
}

#[tokio::test]
async fn push_str_reaches_the_generated_javascript_with_its_argument() {
    let cx = &Cx::default();
    let html = topcoat::view::scope(async {
        view! {
            cx =>
            signal name = String::new();

            <button @click=$(|_e| name.push_str("!"))>"x"</button>
        }
        .unwrap()
        .render(cx)
    })
    .await;

    assert!(html.contains(".push_str("), "{html}");
}

/// `push_str` takes anything that dereferences to a string, so it accepts the
/// owned surrogate an event field yields. `Event::target.value` is a `String`,
/// so this is the call the method mostly exists for; the test above passes a
/// literal, which is borrowed and so never exercised the owned case.
#[tokio::test]
async fn push_str_accepts_the_owned_string_from_an_event() {
    let cx = &Cx::default();
    let html = topcoat::view::scope(async {
        view! {
            cx =>
            signal message = String::new();

            <input
                @input=$(|e: topcoat::runtime::Event| {
                    message.push_str(e.target.value)
                })
            >
        }
        .unwrap()
        .render(cx)
    })
    .await;

    assert!(html.contains(".push_str("), "{html}");
}

#[tokio::test]
async fn logical_and_reaches_the_generated_javascript_lazily() {
    let cx = &Cx::default();
    let html = view! {
        cx =>
        signal a = true;
        signal b = false;

        <p>$(a.get() && b.get())</p>
    }
    .unwrap()
    .render(cx);

    // The right side is a thunk, so the browser only evaluates it when the
    // left side is true. A bare `.and(x)` would evaluate both.
    assert!(html.contains(".and(() =&gt;"), "{html}");
    assert!(html.contains(">false<"), "{html}");
}

#[tokio::test]
async fn logical_or_reaches_the_generated_javascript_lazily() {
    let cx = &Cx::default();
    let html = view! {
        cx =>
        signal a = true;
        signal b = false;

        <p>$(a.get() || b.get())</p>
    }
    .unwrap()
    .render(cx);

    assert!(html.contains(".or(() =&gt;"), "{html}");
    assert!(html.contains(">true<"), "{html}");
}

/// `&&` short-circuits, so the right side of a guarded `unwrap` never runs.
/// Compiling the operator to an eager call would panic here instead, which is
/// why the right side is a closure rather than a value.
#[tokio::test]
async fn logical_and_does_not_evaluate_a_guarded_right_side() {
    let cx = &Cx::default();
    let absent: Option<f64> = None;

    let html = view! {
        cx =>
        <p>$(absent.is_some() && absent.unwrap() > 0.0)</p>
    }
    .unwrap()
    .render(cx);

    assert!(html.contains(">false<"), "{html}");
}

/// The `||` mirror: the right side is skipped once the left side is true.
#[tokio::test]
async fn logical_or_does_not_evaluate_a_settled_right_side() {
    let cx = &Cx::default();
    let absent: Option<f64> = None;

    let html = view! {
        cx =>
        <p>$(absent.is_none() || absent.unwrap() > 0.0)</p>
    }
    .unwrap()
    .render(cx);

    assert!(html.contains(">true<"), "{html}");
}
