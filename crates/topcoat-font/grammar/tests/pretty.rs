use std::fmt::Write;

use topcoat_core_grammar::pretty::{Registry, pretty_print_str};
use topcoat_font_grammar::{font::Font, font_face::FontFace};

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
    let mut registry = Registry::new();
    registry.register_macro::<FontFace>("font_face");
    registry.register_macro::<Font>("font");
    registry
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
fn formats_the_required_descriptors() {
    assert_format_idempotent(
        r#"font_face! { font-family:"Inter";src:local("Inter") }"#,
        "font_face! {\n    font-family: \"Inter\";\n    src: local(\"Inter\");\n}",
    );
}

#[test]
fn formats_every_descriptor() {
    let expected = r#"font_face! {
    font-family: "Inter";
    src: local("Inter"), url("/inter.woff2") format("woff2");
    font-weight: 400 700;
    font-style: oblique 14deg;
    font-display: swap;
    unicode-range: U+0041-005A;
}"#;
    assert_format_idempotent(expected, expected);
}

#[test]
fn preserves_the_written_descriptor_order() {
    assert_format(
        r#"font_face! { src: local("Inter"); font-family: "Inter" }"#,
        "font_face! {\n    src: local(\"Inter\");\n    font-family: \"Inter\";\n}",
    );
}

#[test]
fn preserves_weight_keywords_and_ranges() {
    let expected = r#"font_face! {
    font-family: "Inter";
    src: local("Inter");
    font-weight: normal bold;
}"#;
    assert_format_idempotent(expected, expected);
}

#[test]
fn preserves_style_keywords_and_angles() {
    let expected = r#"font_face! {
    font-family: "Inter";
    src: local("Inter");
    font-style: oblique -12.5deg 40deg;
}"#;
    assert_format_idempotent(expected, expected);
}

#[test]
fn formats_source_hints_and_lists() {
    let expected = r#"font_face! {
    font-family: "Inter";
    src: local("Inter"), url("/inter.woff2") format("woff2") tech("variations");
}"#;
    assert_format_idempotent(expected, expected);
}

#[test]
fn formats_a_unicode_range_list() {
    let expected = r#"font_face! {
    font-family: "Inter";
    src: local("Inter");
    unicode-range: U+0000-00FF, U+0131, U+D800-DFFF;
}"#;
    assert_format_idempotent(expected, expected);
}

#[test]
fn collapses_whitespace_between_descriptors() {
    assert_format(
        "font_face! {\n\n    font-family:    \"Inter\"  ;\n\n\n        src:local(\"Inter\")\n}",
        "font_face! {\n    font-family: \"Inter\";\n    src: local(\"Inter\");\n}",
    );
}

#[test]
fn keeps_a_line_comment_between_descriptors() {
    let expected = r#"font_face! {
    font-family: "Inter";
    // the local copy is preferred
    src: local("Inter");
}"#;
    assert_format_idempotent(expected, expected);
}

#[test]
fn formats_a_font_with_css_blocks() {
    let expected = r#"font! {
    "Inter",
    @font-face {
        src: url("/inter-400.woff2") format("woff2");
        font-weight: 400;
    }
    @font-face {
        src: url("/inter-700.woff2") format("woff2");
        font-weight: 700;
    }
}"#;
    assert_format_idempotent(expected, expected);
}

#[test]
fn formats_a_compact_font_into_blocks() {
    assert_format(
        r#"font! { "Inter",@font-face{src:local("Inter");font-weight:400} }"#,
        "font! {\n    \"Inter\",\n    @font-face {\n        src: local(\"Inter\");\n        font-weight: 400;\n    }\n}",
    );
}

#[test]
fn formats_a_font_with_an_expression() {
    let expected = "font! {\n    \"Inter\",\n    inter_faces()\n}";
    assert_format_idempotent(expected, expected);
}
