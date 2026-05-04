use proc_macro2::extra::DelimSpan;

use crate::{BreakMode, PrettyPrint, Printer};

/// A balanced pair of delimiters (`()`, `[]` or `{}`) that wraps a body of
/// content. The default `pretty_print` implementation handles the open/close
/// tokens, indentation and break behavior; the inner body is supplied by the
/// caller via the `f` closure.
pub trait Delim {
    fn pretty_print(
        &self,
        printer: &mut Printer<'_>,
        break_mode: Option<BreakMode>,
        f: impl FnOnce(&mut Printer<'_>),
    ) {
        printer.move_cursor(self.span().open().start());
        self.open_text().pretty_print(printer);
        printer.move_cursor(self.span().open().end());

        if let Some(break_mode) = break_mode {
            printer.scan_begin(break_mode);
        }
        printer.scan_indent(1);
        printer.scan_break();

        if self.space() {
            " ".pretty_print(printer);
        }

        printer.scan_trivia(false, true);

        f(printer);

        printer.move_cursor(self.span().close().start());
        printer.scan_trivia(true, false);
        printer.scan_indent(-1);
        printer.scan_break();

        if self.space() {
            " ".pretty_print(printer);
        }

        if break_mode.is_some() {
            printer.scan_end();
        }

        self.close_text().pretty_print(printer);
        printer.move_cursor(self.span().close().end());
    }

    #[must_use]
    fn space(&self) -> bool;

    #[must_use]
    fn open_text(&self) -> &'static str;

    #[must_use]
    fn close_text(&self) -> &'static str;

    #[must_use]
    fn span(&self) -> DelimSpan;
}

impl Delim for syn::token::Paren {
    fn space(&self) -> bool {
        false
    }

    fn open_text(&self) -> &'static str {
        "("
    }

    fn close_text(&self) -> &'static str {
        ")"
    }

    fn span(&self) -> DelimSpan {
        self.span
    }
}

impl Delim for syn::token::Bracket {
    fn space(&self) -> bool {
        false
    }

    fn open_text(&self) -> &'static str {
        "["
    }

    fn close_text(&self) -> &'static str {
        "]"
    }

    fn span(&self) -> DelimSpan {
        self.span
    }
}

impl Delim for syn::token::Brace {
    fn space(&self) -> bool {
        true
    }

    fn open_text(&self) -> &'static str {
        "{"
    }

    fn close_text(&self) -> &'static str {
        "}"
    }

    fn span(&self) -> DelimSpan {
        self.span
    }
}
