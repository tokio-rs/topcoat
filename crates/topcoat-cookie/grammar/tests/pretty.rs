use topcoat_cookie_grammar::cookie::Cookie;
use topcoat_core_grammar::common::pretty::{
    Lexer, MARGIN, Macro, PrettyPrint, Printer, Registry, pretty_print_str,
};

/// Formats a macro body such as `(theme = "dark")` at the left margin.
fn print(source: &str) -> String {
    let ast: Macro<Cookie> = syn::parse_str(source).expect("test source must parse");
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
fn joins_the_parts_of_a_name() {
    assert_print(
        r#"(__Host - csrf - token = "x"; Max - Age = 90)"#,
        r#"(__Host-csrf-token = "x"; Max-Age = 90)"#,
    );
}

#[test]
fn spaces_the_attributes() {
    assert_print(
        r#"(theme="dark";Path="/";Secure;SameSite=Lax)"#,
        r#"(theme = "dark"; Path = "/"; Secure; SameSite = Lax)"#,
    );
}

#[test]
fn formats_expressions_inside_parentheses() {
    assert_print(
        "(session = ( token . clone ( ) ); Path = (path))",
        "(session = (token.clone()); Path = (path))",
    );
}

#[test]
fn drops_a_trailing_semicolon_on_one_line() {
    assert_print(
        r#"(theme = "dark"; Secure;)"#,
        r#"(theme = "dark"; Secure)"#,
    );
    assert_print(r#"(theme = "dark";)"#, r#"(theme = "dark")"#);
}

#[test]
fn puts_each_attribute_on_its_own_line_when_too_long() {
    assert_print(
        r#"(__Host-session = (session.token()); Path = "/"; Max-Age = 3600; Secure; HttpOnly; SameSite = Lax)"#,
        r#"(
    __Host-session = (session.token());
    Path = "/";
    Max-Age = 3600;
    Secure;
    HttpOnly;
    SameSite = Lax;
)"#,
    );
}

#[test]
fn copies_attributes_being_typed() {
    assert_print(r#"(theme = "dark"; Path =)"#, r#"(theme = "dark"; Path =)"#);
    assert_print(r#"(theme = "dark"; Pa)"#, r#"(theme = "dark"; Pa)"#);
}

#[test]
fn formats_calls_in_a_file() {
    let mut registry = Registry::new();
    registry.register_function_like::<Cookie>("cookie");
    let formatted = pretty_print_str(&registry, r#"let cookie = cookie!(theme="dark";Secure);"#)
        .unwrap_or_else(|errors| panic!("failed to format: {}", errors[0]));
    assert_eq!(
        formatted,
        r#"let cookie = cookie!(theme = "dark"; Secure);"#
    );
}
