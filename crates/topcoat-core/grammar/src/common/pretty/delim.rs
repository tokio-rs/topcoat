use proc_macro2::extra::DelimSpan;

use crate::common::pretty::{BreakMode, PrettyPrint, Printer};

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
        self.pretty_print_unless_empty(printer, break_mode, false, f);
    }

    /// [`pretty_print`](Self::pretty_print) for a body that may hold nothing.
    ///
    /// The break points inside a delimiter pair are what put the body on lines
    /// of its own. Without a body they have nothing to put there, so a pair
    /// that breaks emits `(` and `)` on lines of their own with nothing
    /// between them. Passing `no_body` closes the pair on one line instead, the
    /// way an empty brace pair collapses to `{}`.
    ///
    /// A comment inside the pair counts as a body: the pair breaks so the
    /// comment keeps its own line.
    fn pretty_print_unless_empty(
        &self,
        printer: &mut Printer<'_>,
        break_mode: Option<BreakMode>,
        no_body: bool,
        f: impl FnOnce(&mut Printer<'_>),
    ) {
        printer.move_cursor(self.span().open().start());
        self.open_text().pretty_print(printer);
        printer.move_cursor(self.span().open().end());

        if no_body && !printer.has_comment_before(self.span().close().start()) {
            self.close_text().pretty_print(printer);
            printer.move_cursor(self.span().close().end());
            printer.skip_trivia();
            return;
        }

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
