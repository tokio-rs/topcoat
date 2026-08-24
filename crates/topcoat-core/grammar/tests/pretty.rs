use std::{fmt::Write, path::Path};

use proc_macro2::LineColumn;
use syn::parse::Parser;
use topcoat_core_grammar::pretty::{BreakMode, Lexer, MARGIN, PrettyPrint, Printer, Registry};

fn diff(expected: &str, actual: &str) -> String {
    let mut output = String::new();
    let expected_lines: Vec<&str> = expected.lines().collect();
    let actual_lines: Vec<&str> = actual.lines().collect();
    let max = expected_lines.len().max(actual_lines.len());
    for i in 0..max {
        let exp = expected_lines.get(i).copied();
        let act = actual_lines.get(i).copied();
        match (exp, act) {
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

/// Formats a sequence of Rust statements at the left margin, separated the way
/// a block separates its statements: one per line, keeping trailing comments,
/// standalone comments and single blank lines.
fn format(source: &str) -> String {
    let stmts = syn::Block::parse_within
        .parse_str(source)
        .unwrap_or_else(|error| panic!("failed to parse input:\n{source}\nerror: {error}"));
    let trivia: Vec<_> = Lexer::new(source).collect();
    let registry = Registry::new();
    let mut printer = Printer::new(&registry, &trivia, MARGIN, 0);

    // The group is forced open so the separating breaks between statements and
    // the breaks that preserve blank lines always fire.
    printer.scan_begin(BreakMode::Consistent);
    printer.scan_force_break();
    printer.scan_break();
    printer.scan_trivia(false, true);
    for (index, stmt) in stmts.iter().enumerate() {
        stmt.pretty_print(&mut printer);
        printer.scan_same_line_trivia();
        if index < stmts.len() - 1 {
            printer.scan_force_break();
            printer.scan_break();
            printer.scan_trivia(true, true);
        }
    }

    // Standalone comments after the last statement get their own lines.
    let end = LineColumn {
        line: usize::MAX,
        column: 0,
    };
    printer.move_cursor(end);
    if printer.has_comment_before(end) {
        printer.scan_force_break();
        printer.scan_break();
        printer.scan_trivia(false, false);
    }
    printer.scan_end();

    printer.eof().trim().to_owned()
}

fn assert_format(input: &str, expected: &str) {
    let result = format(input);
    assert!(
        result == expected,
        "\nformatted output does not match expected\n\n--- diff (expected vs actual) ---\n{}",
        diff(expected, &result),
    );
}

fn assert_format_idempotent(input: &str, expected: &str) {
    assert_format(input, expected);
    // Formatting the expected output again should produce the same result.
    assert_format(expected, expected);
}

fn load_fixture(name: &str) -> (String, String) {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pretty");
    let input = std::fs::read_to_string(base.join(format!("{name}.input")))
        .unwrap_or_else(|e| panic!("failed to read {name}.input: {e}"));
    let expected = std::fs::read_to_string(base.join(format!("{name}.expected")))
        .unwrap_or_else(|e| panic!("failed to read {name}.expected: {e}"));
    (input, expected)
}

macro_rules! fixture_test {
    ($name:ident) => {
        #[test]
        fn $name() {
            let (input, expected) = load_fixture(stringify!($name));
            assert_format_idempotent(input.trim(), expected.trim());
        }
    };
}

fixture_test!(request_handler);
fixture_test!(data_model);
fixture_test!(control_flow);
fixture_test!(iterator_pipelines);
fixture_test!(collections_and_literals);
fixture_test!(generics_and_traits);
fixture_test!(macro_bodies);
fixture_test!(comments_everywhere);
