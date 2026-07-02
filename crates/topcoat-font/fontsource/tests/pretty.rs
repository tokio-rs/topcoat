use std::fmt::Write;

use topcoat_font_fontsource::ast::{font::FontsourceFont, font_face::FontsourceFontFace};
use topcoat_pretty::{Registry, pretty_print_str};

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
    registry.register_macro::<FontsourceFontFace>("fontsource_font_face");
    registry.register_macro::<FontsourceFont>("fontsource_font");
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
fn formats_a_compact_face_inline() {
    assert_format(
        r#"fontsource_font_face!("Inter",weight:400,style:Style::Normal);"#,
        r#"fontsource_font_face!("Inter", weight: 400, style: Style::Normal);"#,
    );
}

#[test]
fn formats_every_face_descriptor() {
    let expected = r#"fontsource_font_face!(
    "Inter",
    weight: 400,
    style: Style::Normal,
    subset: Subset::Latin,
    display: Swap,
    host: Asset,
);"#;
    assert_format_idempotent(expected, expected);
}

#[test]
fn preserves_the_written_face_argument_order() {
    let expected =
        r#"fontsource_font_face!("Inter", host: Asset, style: Style::Normal, weight: 400);"#;
    assert_format_idempotent(expected, expected);
}

#[test]
fn collapses_whitespace_between_face_arguments() {
    assert_format(
        "fontsource_font_face!(  \"Inter\" , weight :  400 ,\n  style: Style::Normal  );",
        r#"fontsource_font_face!("Inter", weight: 400, style: Style::Normal);"#,
    );
}

#[test]
fn keeps_a_line_comment_between_face_arguments() {
    let expected = r#"fontsource_font_face!(
    "Inter",
    // the regular weight
    weight: 400,
    style: Style::Normal,
);"#;
    assert_format_idempotent(expected, expected);
}

#[test]
fn formats_a_compact_font_inline() {
    assert_format(
        r#"fontsource_font!("Inter",weight:[400,700],style:Style::Italic);"#,
        r#"fontsource_font!("Inter", weight: [400, 700], style: Style::Italic);"#,
    );
}

#[test]
fn formats_a_font_with_only_a_family() {
    let expected = r#"fontsource_font!("Lavishly Yours", host: Asset);"#;
    assert_format_idempotent(expected, expected);
}

#[test]
fn formats_every_font_axis_with_lists() {
    let expected = r#"fontsource_font!(
    "Inter",
    weight: [400, 700],
    style: [Style::Normal, Style::Italic],
    subset: [Subset::Latin, Subset::Cyrillic],
    display: Swap,
    host: Asset,
);"#;
    assert_format_idempotent(expected, expected);
}
