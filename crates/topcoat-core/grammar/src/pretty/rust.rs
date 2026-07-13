use quote::ToTokens;
use syn::spanned::Spanned;

use crate::pretty::pretty_print_str;

use super::{PrettyPrint, Printer, TextMode};

fn format_rust_snippet(
    printer: &mut Printer<'_>,
    source: &str,
    prefix: &str,
    suffix: &str,
    indent: isize,
) -> String {
    let mut input = String::new();
    for _ in 0..indent {
        input.push_str("const _: () = {");
    }
    input.push_str(prefix);
    input.push_str(source);
    input.push_str(suffix);
    for _ in 0..indent {
        input.push_str("};");
    }

    let file = syn::parse_file(&input).expect("failed to parse rust snippet for formatting");
    let formatted = prettyplease::unparse(&file);
    let formatted = pretty_print_str(printer.registry(), &formatted).unwrap();

    let mut stripped = formatted.trim();
    for _ in 0..indent {
        stripped = stripped.strip_prefix("const _: () = {").unwrap();
        stripped = stripped.strip_suffix("};").unwrap();
        stripped = stripped.trim();
    }
    let stripped = stripped.strip_prefix(prefix).unwrap();
    let stripped = stripped.strip_suffix(suffix).unwrap();
    stripped.to_owned()
}

impl PrettyPrint for syn::Expr {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        let source_text = self
            .span()
            .source_text()
            .expect("cannot pretty print rust expr without source text");
        let output = format_rust_snippet(
            printer,
            &source_text,
            "const _: () = ",
            ";",
            printer.current_indent(),
        );
        output.pretty_print(printer);
    }
}

impl PrettyPrint for syn::Pat {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        let source_text = self
            .span()
            .source_text()
            .expect("cannot pretty print rust expr without source text");
        let output = format_rust_snippet(
            printer,
            &source_text,
            "const _: () = matches!(x, ",
            ");",
            printer.current_indent(),
        );
        output.pretty_print(printer);
    }
}

impl PrettyPrint for syn::ExprLet {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.let_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.pat.pretty_print(printer);
        " ".pretty_print(printer);
        self.eq_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.expr.pretty_print(printer);
    }
}

impl PrettyPrint for syn::Attribute {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        if self.meta.path().is_ident("doc") {
            // Docs comments are treated as regular comments by the pretty printer.
            return;
        }

        self.pound_token.pretty_print(printer);
        if let syn::AttrStyle::Inner(not) = &self.style {
            not.pretty_print(printer);
        }
        if let Some(source_text) = self.bracket_token.span.span().source_text() {
            source_text.pretty_print(printer);
        }
        printer.scan_same_line_trivia();
        printer.scan_break();
        " ".pretty_print(printer);
    }
}

impl PrettyPrint for syn::token::PathSep {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        printer.move_cursor(self.span().start());
        printer.scan_text("::".into(), TextMode::Always);
        printer.move_cursor(self.span().end());
    }
}

impl PrettyPrint for syn::Path {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        if let Some(leading_colon) = &self.leading_colon {
            leading_colon.pretty_print(printer);
        }

        for pair in self.segments.pairs() {
            let segment = pair.value();
            segment.ident.pretty_print(printer);

            match &segment.arguments {
                syn::PathArguments::None => {}
                syn::PathArguments::AngleBracketed(args) => {
                    args.pretty_print(printer);
                }
                syn::PathArguments::Parenthesized(args) => {
                    args.pretty_print(printer);
                }
            }

            if let Some(punct) = pair.punct() {
                punct.pretty_print(printer);
            }
        }
    }
}

impl PrettyPrint for syn::AngleBracketedGenericArguments {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.lt_token.pretty_print(printer);

        for pair in self.args.pairs() {
            pair.value().pretty_print(printer);

            if let Some(punct) = pair.punct() {
                printer.scan_no_break_trivia();
                punct.pretty_print(printer);
                printer.scan_trivia(true, true);
                " ".pretty_print(printer);
            }
        }

        self.gt_token.pretty_print(printer);
    }
}

impl PrettyPrint for syn::GenericArgument {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        // Use to_token_stream for generic arguments
        printer.move_cursor(self.span().start());
        printer.scan_text(self.to_token_stream().to_string().into(), TextMode::Always);
        printer.move_cursor(self.span().end());
    }
}

impl PrettyPrint for syn::ParenthesizedGenericArguments {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        use super::Delim;

        self.paren_token.pretty_print(printer, None, |printer| {
            for pair in self.inputs.pairs() {
                pair.value().pretty_print(printer);

                if let Some(punct) = pair.punct() {
                    printer.scan_no_break_trivia();
                    punct.pretty_print(printer);
                    printer.scan_trivia(true, true);
                    " ".pretty_print(printer);
                }
            }
        });

        // Handle return type if present
        match &self.output {
            syn::ReturnType::Default => {}
            syn::ReturnType::Type(arrow, ty) => {
                " ".pretty_print(printer);
                arrow.pretty_print(printer);
                " ".pretty_print(printer);
                ty.pretty_print(printer);
            }
        }
    }
}

impl PrettyPrint for syn::Type {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        // Use to_token_stream for types
        printer.move_cursor(self.span().start());
        printer.scan_text(self.to_token_stream().to_string().into(), TextMode::Always);
        printer.move_cursor(self.span().end());
    }
}

impl PrettyPrint for syn::token::RArrow {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        printer.move_cursor(self.span().start());
        printer.scan_text("->".into(), TextMode::Always);
        printer.move_cursor(self.span().end());
    }
}
