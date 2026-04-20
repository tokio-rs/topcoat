use std::{
    io::Write,
    process::{Command, Stdio},
};

use quote::ToTokens;
use syn::spanned::Spanned;

use crate::pretty::pretty_print_rust_str;

use super::{PrettyPrint, Printer, TextMode};

impl PrettyPrint for syn::Expr {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        let source_text = self
            .span()
            .source_text()
            .expect("cannot pretty print rust expr without source text");
        let command = Command::new("rustfmt")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("failed to run rustfmt on nested rust expression");
        let mut stdin = command.stdin.as_ref().expect("command must has stdin");
        println!("{}", printer.current_indent());
        for _ in 0..printer.current_indent() {
            stdin.write_all("const _: () = {".as_bytes()).unwrap();
        }
        stdin.write_all("const _: () = ".as_bytes()).unwrap();
        stdin.write_all(source_text.as_bytes()).unwrap();
        stdin.write_all(";".as_bytes()).unwrap();
        for _ in 0..printer.current_indent() {
            stdin.write_all("};".as_bytes()).unwrap();
        }
        let output = command
            .wait_with_output()
            .expect("failed to run rustfmt on nested rust expression");
        let output = String::from_utf8(output.stdout).expect("rustfmt output must be utf8");
        let output = pretty_print_rust_str(&output).unwrap();
        let mut stripped = output.trim();
        for _ in 0..printer.current_indent() {
            stripped = stripped.strip_prefix("const _: () = {").unwrap();
            stripped = stripped.strip_suffix("};").unwrap();
            stripped = stripped.trim();
        }
        let stripped = stripped.strip_prefix("const _: () = ").unwrap();
        let stripped = stripped.strip_suffix(";").unwrap();
        let output = stripped.to_owned();
        output.pretty_print(printer);
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
