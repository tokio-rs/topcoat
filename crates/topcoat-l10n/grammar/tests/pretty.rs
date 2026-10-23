use syn::parse::Parse;
use topcoat_core_grammar::pretty::{
    Lexer, MARGIN, Macro, PrettyPrint, Printer, Registry, pretty_print_str,
};
use topcoat_l10n_grammar::fluent::{Messages, Translate};

/// Formats a macro body such as `(greeting, name)` at the left margin.
fn print<T>(source: &str) -> String
where
    T: Parse + PrettyPrint,
{
    let ast: Macro<T> = syn::parse_str(source).expect("test source must parse");
    let trivia: Vec<_> = Lexer::new(source).collect();
    let registry = Registry::new();
    let mut printer = Printer::new(&registry, &trivia, MARGIN, 0);
    ast.pretty_print(&mut printer);
    printer.eof()
}

/// Formatting `input` must produce `expected`, and formatting `expected` again must
/// leave it unchanged.
fn assert_print<T>(input: &str, expected: &str)
where
    T: Parse + PrettyPrint,
{
    assert_eq!(print::<T>(input), expected);
    assert_eq!(print::<T>(expected), expected);
}

fn registry() -> Registry {
    let mut registry = Registry::new();
    registry.register_macro::<Translate>("t");
    registry.register_macro::<Messages>("messages");
    registry
}

/// Formats every registered macro in a Rust source file.
fn format_file(input: &str) -> String {
    pretty_print_str(&registry(), input).unwrap_or_else(|errors| {
        panic!("failed to format:\n{input}\nerror: {}", errors[0]);
    })
}

#[test]
fn joins_the_parts_of_a_key() {
    assert_print::<Translate>(
        "(password - field . min - length)",
        "(password-field.min-length)",
    );
    assert_print::<Translate>("(step - 2a)", "(step-2a)");
    assert_print::<Translate>(r#"("item-1e" . label)"#, r#"("item-1e".label)"#);
}

#[test]
fn spaces_the_arguments() {
    assert_print::<Translate>(
        "(greeting,name=user . name,count)",
        "(greeting, name = user.name, count)",
    );
    assert_print::<Translate>(
        r#"(greeting, "user-name"=name)"#,
        r#"(greeting, "user-name" = name)"#,
    );
}

#[test]
fn drops_a_trailing_comma_on_one_line() {
    assert_print::<Translate>("(greeting, name,)", "(greeting, name)");
    assert_print::<Translate>("(greeting,)", "(greeting)");
}

#[test]
fn puts_each_argument_on_its_own_line_when_too_long() {
    assert_print::<Translate>(
        "(order-confirmation-summary, customer_name = order.customer.name(), total = order.total(), count)",
        "(
    order-confirmation-summary,
    customer_name = order.customer.name(),
    total = order.total(),
    count,
)",
    );
}

#[test]
fn copies_incomplete_values() {
    assert_print::<Translate>("(greeting, name =)", "(greeting, name =)");
    assert_print::<Translate>("(greeting, name = a +, count)", "(greeting, name = a +, count)");
}

#[test]
fn keeps_doc_comments_on_their_own_lines() {
    assert_print::<Translate>(
        "(
    /// Shown inside the empty field.
    password - field . placeholder,
    /// Minimum number of characters.
    min=8
)",
        "(
    /// Shown inside the empty field.
    password-field.placeholder,
    /// Minimum number of characters.
    min = 8,
)",
    );
}

#[test]
fn spaces_the_visibility_of_messages() {
    assert_eq!(
        format_file(r#"messages!(pub(crate)"{mod}/l10n/{locale}.ftl");"#),
        r#"messages!(pub(crate) "{mod}/l10n/{locale}.ftl");"#,
    );
    assert_eq!(
        format_file(r#"messages!(pub   "l10n/{locale}.ftl");"#),
        r#"messages!(pub "l10n/{locale}.ftl");"#,
    );
}
