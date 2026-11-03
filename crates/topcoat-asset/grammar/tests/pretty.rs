use topcoat_asset_grammar::asset::Asset;
use topcoat_core_grammar::common::pretty::{
    Lexer, MARGIN, Macro, PrettyPrint, Printer, Registry, pretty_print_str,
};

/// Formats a macro body such as `("./logo.png", rename: "logo")` at the left margin.
fn print(source: &str) -> String {
    let ast: Macro<Asset> = syn::parse_str(source).expect("test source must parse");
    let trivia: Vec<_> = Lexer::new(source).collect();
    let registry = Registry::new();
    let mut printer = Printer::new(&registry, &trivia, MARGIN, 0);
    ast.pretty_print(&mut printer);
    printer.eof()
}

/// Formatting `input` must produce `expected`, and formatting `expected` again must
/// leave it unchanged.
fn assert_print(input: &str, expected: &str) {
    assert_eq!(print(input), expected);
    assert_eq!(print(expected), expected);
}

#[test]
fn spaces_the_options() {
    assert_print(
        r#"("./logo.png",rename :"logo",extension:"svg")"#,
        r#"("./logo.png", rename: "logo", extension: "svg")"#,
    );
}

#[test]
fn formats_option_values_as_expressions() {
    assert_print(
        r#"("./logo.png", rename: names :: LOGO)"#,
        r#"("./logo.png", rename: names::LOGO)"#,
    );
}

#[test]
fn drops_a_trailing_comma_on_one_line() {
    assert_print(
        r#"("./logo.png", rename: "logo",)"#,
        r#"("./logo.png", rename: "logo")"#,
    );
    assert_print(r#"("./logo.png",)"#, r#"("./logo.png")"#);
}

#[test]
fn puts_each_option_on_its_own_line_when_too_long() {
    assert_print(
        r#"("https://example.com/fonts/inter-variable.woff2", rename: "inter", checksum: "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")"#,
        r#"(
    "https://example.com/fonts/inter-variable.woff2",
    rename: "inter",
    checksum: "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
)"#,
    );
}

#[test]
fn copies_options_being_typed() {
    assert_print(r#"("./logo.png", rename:)"#, r#"("./logo.png", rename:)"#);
    assert_print(r#"("./logo.png", ren)"#, r#"("./logo.png", ren)"#);
}

#[test]
fn formats_calls_in_a_file() {
    let mut registry = Registry::new();
    registry.register_function_like::<Asset>("asset");
    let formatted = pretty_print_str(
        &registry,
        r#"const LOGO: Asset = asset!("./logo.png",rename:"logo");"#,
    )
    .unwrap_or_else(|errors| panic!("failed to format: {}", errors[0]));
    assert_eq!(
        formatted,
        r#"const LOGO: Asset = asset!("./logo.png", rename: "logo");"#
    );
}
