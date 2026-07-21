use std::fmt::Write;
use std::path::Path;

use topcoat_core_grammar::pretty::{Registry, pretty_print_str};
use topcoat_view_grammar::view::View;

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

fn registry() -> Registry {
    let mut registry = Registry::new();
    registry.register_macro::<View>("view");
    registry.register_macro::<topcoat_view_grammar::attributes::Attributes>("attributes");
    registry.register_macro::<topcoat_view_grammar::class::Class>("class");
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

// -- Basic elements ----------------------------------------------------------

fixture_test!(single_element);
fixture_test!(void_elements);
fixture_test!(nested_elements);
fixture_test!(element_with_text);
fixture_test!(empty_element_wrapped_attributes);
fixture_test!(empty_element_interior_comment);

// -- Attributes --------------------------------------------------------------

fixture_test!(attributes_short);
fixture_test!(attributes_long);
fixture_test!(attributes_overflow);
fixture_test!(attributes_macro_short);
fixture_test!(attributes_macro_long);
fixture_test!(attribute_expr_value);

// -- Text nodes --------------------------------------------------------------

fixture_test!(text_nodes);

// -- Expressions -------------------------------------------------------------

fixture_test!(inline_expressions);

// -- Control flow ------------------------------------------------------------

fixture_test!(if_else);
fixture_test!(if_else_if_else);
fixture_test!(for_loop);
fixture_test!(match_expr);

// -- Components --------------------------------------------------------------

fixture_test!(component_empty);
fixture_test!(component_with_child);
fixture_test!(component_with_children);

// -- Explicit context --------------------------------------------------------

fixture_test!(explicit_cx);
fixture_test!(explicit_cx_multiline);

// -- Class lists --------------------------------------------------------------

fixture_test!(class_macro_short);
fixture_test!(class_macro_long);

// -- Local bindings ------------------------------------------------------------

fixture_test!(local_binding);
fixture_test!(local_binding_typed);

// -- DOCTYPE -----------------------------------------------------------------

fixture_test!(doctype);

// -- Comments ----------------------------------------------------------------

fixture_test!(line_comments);
fixture_test!(block_comments);

// -- Complex / realistic -----------------------------------------------------

fixture_test!(full_page);
fixture_test!(deeply_nested);
fixture_test!(comments_everywhere);
