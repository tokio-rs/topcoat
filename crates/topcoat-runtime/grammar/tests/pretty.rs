use topcoat_core_grammar::pretty::{Registry, pretty_print_str};
use topcoat_runtime_grammar::{
    expr::ExprInput, procedure::ProcedureAttr, record::RecordAttr, shard::ShardAttr,
};

fn registry() -> Registry {
    let mut registry = Registry::new();
    registry
        .register_function_like::<ExprInput>("expr")
        .register_function_like::<syn::Item>("wrap")
        .register_function_like::<syn::Expr>("value")
        .register_attribute::<ProcedureAttr>("procedure")
        .register_attribute::<ShardAttr>("shard")
        .register_attribute::<RecordAttr>("record");
    registry
}

fn format(input: &str) -> String {
    pretty_print_str(&registry(), input).unwrap_or_else(|errors| {
        panic!(
            "failed to format input:\n{input}\nerror: {}",
            errors.first().unwrap()
        )
    })
}

/// Formatting `input` must produce `expected`, and formatting `expected` again
/// must be a fixed point.
fn assert_format(input: &str, expected: &str) {
    assert_eq!(format(input), expected, "formatting:\n{input}");
    assert_eq!(format(expected), expected, "reformatting:\n{expected}");
}

// `rustfmt` lays out `expr!(...)` at file level, so these calls are formatted
// inside a macro body that `rustfmt` does not reach.

#[test]
fn formats_the_expression() {
    assert_format(
        "value! { label(expr!(count.get()  *  2)) }",
        "value! { label(expr!(count.get() * 2)) }",
    );
}

#[test]
fn drops_a_trailing_comma_on_one_line() {
    assert_format(
        "value! { label(expr!(count.get() * 2,)) }",
        "value! { label(expr!(count.get() * 2)) }",
    );
}

#[test]
fn keeps_a_trailing_comma_on_a_split_call() {
    assert_format(
        "value! { label(expr!(if validated.get() { first_extremely_long_message } else { second_extremely_long_message },)) }",
        "value! {\n    label(\n        expr!(\n            if validated.get() {\n                first_extremely_long_message\n            } else {\n                second_extremely_long_message\n            },\n        ),\n    )\n}",
    );
}

#[test]
fn formats_endpoint_paths() {
    // `rustfmt` lays out these attributes at file level, so they are
    // formatted inside a macro body that `rustfmt` does not reach.
    let output = format("wrap! { #[procedure(  \"/api/double\"  )] async fn double() {} }");
    assert!(output.contains("#[procedure(\"/api/double\")]"), "{output}");
    let output = format("wrap! { #[shard(  \"/search\"  )] async fn search() {} }");
    assert!(output.contains("#[shard(\"/search\")]"), "{output}");
}

#[test]
fn formats_empty_record_arguments() {
    let output = format("wrap! { #[record(  )] struct Point { x: f64 } }");
    assert!(output.contains("#[record()]"), "{output}");
}
