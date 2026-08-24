use super::common;
use crate::pretty::{PrettyPrint, Printer};

impl PrettyPrint for syn::Block {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        common::statement_braces(printer, &self.brace_token, &self.stmts);
    }
}

impl PrettyPrint for syn::Stmt {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        match self {
            Self::Local(local) => local.pretty_print(printer),
            Self::Item(item) => item.pretty_print(printer),
            Self::Expr(expr, semi) => {
                expr.pretty_print(printer);
                semi.pretty_print(printer);
            }
            Self::Macro(stmt) => stmt.pretty_print(printer),
        }
    }
}

impl PrettyPrint for syn::StmtMacro {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.mac.pretty_print(printer);
        self.semi_token.pretty_print(printer);
    }
}

impl PrettyPrint for syn::Local {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.let_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.pat.pretty_print(printer);
        if let Some(init) = &self.init {
            " ".pretty_print(printer);
            init.eq_token.pretty_print(printer);
            " ".pretty_print(printer);
            init.expr.pretty_print(printer);
            if let Some((else_token, diverge)) = &init.diverge {
                " ".pretty_print(printer);
                else_token.pretty_print(printer);
                " ".pretty_print(printer);
                diverge.pretty_print(printer);
            }
        }
        self.semi_token.pretty_print(printer);
    }
}

#[cfg(test)]
mod tests {
    use super::super::common::tests::format;

    fn block(source: &str) -> String {
        format::<syn::Block>(source)
    }

    #[test]
    fn empty_block() {
        assert_eq!(block("{}"), "{}");
        assert_eq!(block("{   }"), "{}");
    }

    #[test]
    fn statements_sit_on_their_own_lines() {
        assert_eq!(
            block("{ first(); second(); third() }"),
            "{\n    first();\n    second();\n    third()\n}",
        );
    }

    #[test]
    fn let_bindings() {
        assert_eq!(block("{ let x = 1; }"), "{\n    let x = 1;\n}");
        assert_eq!(
            block("{ let mut total: u64 = 0; }"),
            "{\n    let mut total: u64 = 0;\n}",
        );
        assert_eq!(
            block("{ let (a, b) = pair; }"),
            "{\n    let (a, b) = pair;\n}",
        );
    }

    #[test]
    fn let_else() {
        assert_eq!(
            block("{ let Some(value) = optional else { return }; }"),
            "{\n    let Some(value) = optional else {\n        return\n    };\n}",
        );
    }

    #[test]
    fn nested_blocks_indent() {
        assert_eq!(
            block("{ if ready { start(); } done() }"),
            "{\n    if ready {\n        start();\n    }\n    done()\n}",
        );
    }

    #[test]
    fn macro_statement() {
        assert_eq!(
            block("{ println!(\"{}\", value); }"),
            "{\n    println!(\"{}\", value);\n}",
        );
    }

    #[test]
    fn trailing_comment_stays_on_its_line() {
        assert_eq!(
            block("{ first(); // done\n    second() }"),
            "{\n    first(); // done\n    second()\n}",
        );
    }

    #[test]
    fn standalone_comment_between_statements() {
        assert_eq!(
            block("{ first();\n    // explain the next step\n    second() }"),
            "{\n    first();\n    // explain the next step\n    second()\n}",
        );
    }

    #[test]
    fn comment_only_block_is_kept() {
        assert_eq!(block("{ /* nothing */ }"), "{ /* nothing */\n}");
    }

    #[test]
    fn blank_lines_collapse_to_one() {
        assert_eq!(
            block("{ first();\n\n\n\n    second(); }"),
            "{\n    first();\n\n    second();\n}",
        );
    }

    #[test]
    fn leading_blank_lines_are_dropped() {
        assert_eq!(block("{\n\n    only(); }"), "{\n    only();\n}");
    }

    #[test]
    fn line_comment_before_closing_brace() {
        assert_eq!(
            block("{ work();\n    // trailing note\n}"),
            "{\n    work();\n    // trailing note\n}",
        );
    }

    #[test]
    fn statement_spacing_is_normalized() {
        assert_eq!(
            block("{let x=compute( a,b );use_it(x);}"),
            "{\n    let x = compute(a, b);\n    use_it(x);\n}",
        );
    }

    #[test]
    fn long_let_initializer_breaks_inside_call() {
        assert_eq!(
            block(
                "{ let result = compute_something(first_long_argument_value, second_long_argument_value, third_one); }"
            ),
            "{\n    let result = compute_something(\n        first_long_argument_value,\n        second_long_argument_value,\n        third_one,\n    );\n}",
        );
    }

    #[test]
    fn doc_comment_reaches_the_output() {
        assert_eq!(
            block("{\n    /// Explains the function.\n    fn helper() {}\n}"),
            "{\n    /// Explains the function.\n    fn helper() {}\n}",
        );
    }
}
