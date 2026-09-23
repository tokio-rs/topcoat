use proc_macro2::extra::DelimSpan;

use crate::pretty::{BreakMode, PrettyPrint, Printer};

/// A pair of delimiters (`()`, `[]`, or `{}`) around a body.
///
/// The provided [`pretty_print`](Self::pretty_print) method prints the
/// delimiters, the indentation, and the line breaks, and calls a closure to
/// print the body.
pub trait Delim {
    /// Prints the delimiters with the body printed by `f` between them.
    ///
    /// When `break_mode` is set, the delimiters and body form a group with
    /// that mode. The body is indented one level when the group breaks.
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

    /// Returns whether a space separates the body from the delimiters when
    /// they print on one line, as in `{ a }`.
    #[must_use]
    fn space(&self) -> bool;

    /// Returns the opening delimiter, like `(`.
    #[must_use]
    fn open_text(&self) -> &'static str;

    /// Returns the closing delimiter, like `)`.
    #[must_use]
    fn close_text(&self) -> &'static str;

    /// Returns the source span of the delimiters.
    #[must_use]
    fn span(&self) -> DelimSpan;
}

/// Wraps a [`Delim`] to suppress the spacing it would otherwise add around its
/// body, so a brace pair can print as `{}` or `{a, b}` while still reusing the
/// delimiter's cursor and trivia handling.
pub(crate) struct Unspaced<'a, D>(pub(crate) &'a D);

impl<D> Delim for Unspaced<'_, D>
where
    D: Delim,
{
    fn space(&self) -> bool {
        false
    }

    fn open_text(&self) -> &'static str {
        self.0.open_text()
    }

    fn close_text(&self) -> &'static str {
        self.0.close_text()
    }

    fn span(&self) -> DelimSpan {
        self.0.span()
    }
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
