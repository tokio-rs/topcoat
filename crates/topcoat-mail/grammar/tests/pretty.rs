use std::fmt::Write;

use topcoat_core_grammar::pretty::{Registry, pretty_print_str};
use topcoat_mail_grammar::mail::Mail;

fn diff(expected: &str, actual: &str) -> String {
    let mut output = String::new();
    let expected_lines: Vec<&str> = expected.lines().collect();
    let actual_lines: Vec<&str> = actual.lines().collect();
    let max = expected_lines.len().max(actual_lines.len());
    for i in 0..max {
        match (expected_lines.get(i).copied(), actual_lines.get(i).copied()) {
            (Some(e), Some(a)) if e == a => {
                let _ = writeln!(output, "   {e}");
            }
            (Some(e), Some(a)) => {
                let _ = writeln!(output, "  -{e}");
                let _ = writeln!(output, "  +{a}");
            }
            (Some(e), None) => {
                let _ = writeln!(output, "  -{e}");
            }
            (None, Some(a)) => {
                let _ = writeln!(output, "  +{a}");
            }
            (None, None) => {}
        }
    }
    output
}

fn registry() -> Registry {
    Registry::one::<Mail>("mail")
}

fn assert_format(input: &str, expected: &str) {
    let result = pretty_print_str(&registry(), input).unwrap_or_else(|errors| {
        panic!(
            "failed to parse input:\n{input}\nerror: {}",
            errors.first().unwrap()
        );
    });
    assert!(
        result == expected,
        "\nformatted output does not match expected\n\n--- diff (expected vs actual) ---\n{}",
        diff(expected, &result),
    );
}

/// Formatting `input` must produce `expected`, and formatting `expected` again
/// must be a fixed point.
fn assert_format_idempotent(input: &str, expected: &str) {
    assert_format(input, expected);
    assert_format(expected, expected);
}

#[test]
fn keeps_an_empty_body_collapsed() {
    let expected = "mail! {}";
    assert_format_idempotent(expected, expected);
}

#[test]
fn keeps_a_short_body_on_one_line() {
    let expected = r#"mail! { subject: "Hello" }"#;
    assert_format_idempotent(expected, expected);
}

#[test]
fn normalizes_spacing_within_a_field() {
    assert_format(
        r#"mail! {subject   :"Hello"}"#,
        r#"mail! { subject: "Hello" }"#,
    );
}

#[test]
fn breaks_a_long_body_one_field_per_line() {
    assert_format_idempotent(
        r#"mail! { from: "ada@example.com", to: ["bob@example.com"], subject: "Analytical engines", text: "The engine weaves." }"#,
        r#"mail! {
    from: "ada@example.com",
    to: ["bob@example.com"],
    subject: "Analytical engines",
    text: "The engine weaves.",
}"#,
    );
}

#[test]
fn keeps_a_short_list_on_one_line() {
    let expected = r#"mail! {
    to: ["bob@example.com", "grace@example.com"],
    subject: "Analytical engines and their patterns",
}"#;
    assert_format_idempotent(expected, expected);
}

#[test]
fn formats_the_html_view_body() {
    assert_format_idempotent(
        r#"mail! { subject: "Hello", html: {<div><p>"The engine weaves algebraic patterns."</p><p>"Yours, Ada"</p></div>} }"#,
        r#"mail! {
    subject: "Hello",
    html: {
        <div>
            <p>"The engine weaves algebraic patterns."</p>
            <p>"Yours, Ada"</p>
        </div>
    },
}"#,
    );
}

#[test]
fn keeps_a_short_html_body_inline() {
    let expected = r#"mail! { html: { <p>"Hi"</p> } }"#;
    assert_format_idempotent(expected, expected);
}

#[test]
fn preserves_a_leading_cx_argument() {
    let expected = r#"mail! {
    subject: "Hello",
    html: {
        cx =>
        <p>
            "Hello, "
            (name)
            "!"
        </p>
    },
}"#;
    assert_format_idempotent(expected, expected);
}

#[test]
fn keeps_a_line_comment_between_fields() {
    let expected = r#"mail! {
    subject: "Hello",
    // delivered to the archive as well
    bcc: ["archive@example.com"],
}"#;
    assert_format_idempotent(expected, expected);
}
