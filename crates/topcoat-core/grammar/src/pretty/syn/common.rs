use proc_macro2::Span;
use quote::ToTokens;
use syn::spanned::Spanned;

use crate::pretty::{BreakMode, PrettyPrint, Printer, TextMode};

/// Emits `text` for a token whose original source occupies `span`, keeping the
/// printer's cursor in sync so trivia around the token stays anchored.
pub(super) fn token(printer: &mut Printer<'_>, text: &'static str, span: Span) {
    printer.move_cursor(span.start());
    printer.scan_text(text.into(), TextMode::Always);
    printer.move_cursor(span.end());
}

/// Prints `tokens` exactly as written in the source, comments included, and
/// drops the trivia the copied text already contains. Serves as the fallback
/// for constructs without a dedicated layout.
pub(super) fn verbatim(printer: &mut Printer<'_>, tokens: &impl ToTokens) {
    verbatim_span(printer, tokens.span(), || {
        tokens.to_token_stream().to_string()
    });
}

/// Prints the source text covered by `span` unchanged, falling back to
/// `fallback` when no source text is available.
pub(super) fn verbatim_span(
    printer: &mut Printer<'_>,
    span: Span,
    fallback: impl FnOnce() -> String,
) {
    let text = span.source_text().unwrap_or_else(fallback);
    printer.move_cursor(span.start());
    printer.scan_text(text.into(), TextMode::Always);
    printer.move_cursor(span.end());
    printer.skip_trivia();
}

/// Prints a separating comma: the source's own token when it exists, so the
/// cursor tracks it, or a bare `,` when the source omitted it.
pub(super) fn comma(printer: &mut Printer<'_>, punct: Option<&syn::Token![,]>) {
    if let Some(punct) = punct {
        punct.pretty_print(printer);
    } else {
        printer.scan_text(",".into(), TextMode::Always);
        printer.advance_cursor(",");
    }
}

/// Whether a brace pair without entries encloses nothing but whitespace, so it
/// can collapse to `{}`.
pub(super) fn braces_are_empty(
    printer: &Printer<'_>,
    brace: &syn::token::Brace,
    no_entries: bool,
) -> bool {
    no_entries && !printer.has_comment_before(brace.span.close().start())
}

/// Prints an empty brace pair as `{}`, dropping the whitespace between the
/// braces.
pub(super) fn empty_braces(printer: &mut Printer<'_>, brace: &syn::token::Brace) {
    printer.move_cursor(brace.span.open().start());
    printer.scan_text("{}".into(), TextMode::Always);
    printer.move_cursor(brace.span.close().end());
    printer.skip_trivia();
}

/// Prints a brace pair whose body always breaks onto its own lines, with the
/// entries supplied by `f`. Trailing comments stay on their line; standalone
/// comments before the closing brace get their own lines.
fn braced_lines(
    printer: &mut Printer<'_>,
    brace: &syn::token::Brace,
    is_empty: bool,
    f: impl FnOnce(&mut Printer<'_>),
) {
    if braces_are_empty(printer, brace, is_empty) {
        empty_braces(printer, brace);
        return;
    }

    token(printer, "{", brace.span.open());
    printer.scan_begin(BreakMode::Consistent);
    printer.scan_indent(1);
    printer.scan_same_line_trivia();

    if !is_empty {
        printer.scan_force_break();
        printer.scan_break();
        printer.scan_trivia(false, true);
        f(printer);
        printer.scan_same_line_trivia();
    }

    let close = brace.span.close();
    printer.move_cursor(close.start());
    if printer.has_comment_before(close.start()) {
        printer.scan_force_break();
        printer.scan_break();
        printer.scan_trivia(false, false);
    }
    printer.skip_trivia();
    printer.scan_indent(-1);
    printer.scan_force_break();
    printer.scan_break();
    printer.scan_end();
    printer.scan_text("}".into(), TextMode::Always);
    printer.move_cursor(close.end());
}

/// Prints a brace-delimited body whose entries always sit on their own lines:
/// a block's statements, an impl's items, or a match's arms. Standalone
/// comments and single blank lines between entries are kept.
pub(super) fn statement_braces<T>(
    printer: &mut Printer<'_>,
    brace: &syn::token::Brace,
    entries: &[T],
) where
    T: PrettyPrint,
{
    braced_lines(printer, brace, entries.is_empty(), |printer| {
        for (index, entry) in entries.iter().enumerate() {
            entry.pretty_print(printer);
            if index < entries.len() - 1 {
                printer.scan_same_line_trivia();
                printer.scan_force_break();
                printer.scan_break();
                printer.scan_trivia(true, true);
            }
        }
    });
}

/// Prints a brace-delimited comma list whose entries always sit on their own
/// lines: a struct's fields or an enum's variants.
pub(super) fn forced_comma_braces<T>(
    printer: &mut Printer<'_>,
    brace: &syn::token::Brace,
    items: &syn::punctuated::Punctuated<T, syn::Token![,]>,
) where
    T: PrettyPrint,
{
    braced_lines(printer, brace, items.is_empty(), |printer| {
        for (index, pair) in items.pairs().enumerate() {
            pair.value().pretty_print(printer);
            if pair.punct().is_some() {
                printer.scan_no_break_trivia();
            }
            comma(printer, pair.punct().copied());
            if index < items.len() - 1 {
                printer.scan_same_line_trivia();
                printer.scan_force_break();
                printer.scan_break();
                printer.scan_trivia(true, true);
            }
        }
    });
}

/// Prints a comma-separated list, adding a trailing comma that only renders
/// when the surrounding group breaks. A `tail` entry (a struct literal's
/// `..rest`, a signature's variadic `...`) is printed last without one.
pub(super) fn comma_separated<T>(
    printer: &mut Printer<'_>,
    items: &syn::punctuated::Punctuated<T, syn::Token![,]>,
    tail: Option<&dyn PrettyPrint>,
) where
    T: PrettyPrint,
{
    for (index, pair) in items.pairs().enumerate() {
        pair.value().pretty_print(printer);
        if pair.punct().is_some() {
            printer.scan_no_break_trivia();
        }
        if index == items.len() - 1 && tail.is_none() {
            printer.scan_text(",".into(), TextMode::Break);
            printer.advance_cursor(",");
        } else {
            comma(printer, pair.punct().copied());
            printer.scan_same_line_trivia();
            printer.scan_break();
            " ".pretty_print(printer);
            printer.scan_trivia(true, true);
        }
    }
    if let Some(tail) = tail {
        tail.pretty_print(printer);
    }
}

impl<T> PrettyPrint for syn::punctuated::Punctuated<T, syn::Token![,]>
where
    T: PrettyPrint,
{
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        comma_separated(printer, self, None);
    }
}

/// Prints a punctuated list whose separators sit between spaces (`A + B`,
/// `x | y`), with a break point before each separator. The caller provides the
/// surrounding group.
pub(super) fn space_separated<T, P>(
    printer: &mut Printer<'_>,
    items: &syn::punctuated::Punctuated<T, P>,
) where
    T: PrettyPrint,
    P: PrettyPrint,
{
    for pair in items.pairs() {
        pair.value().pretty_print(printer);
        if let Some(punct) = pair.punct() {
            printer.scan_same_line_trivia();
            printer.scan_break();
            " ".pretty_print(printer);
            punct.pretty_print(printer);
            " ".pretty_print(printer);
            printer.scan_trivia(true, true);
        }
    }
}

#[cfg(test)]
pub(super) mod tests {
    use syn::parse::Parse;

    use crate::pretty::{Lexer, MARGIN, PrettyPrint, Printer, Registry};

    /// Formats `source` parsed as `T` at the left margin with an empty macro
    /// registry.
    pub(in crate::pretty::syn) fn format<T>(source: &str) -> String
    where
        T: Parse + PrettyPrint,
    {
        format_with_registry::<T>(&Registry::new(), source)
    }

    /// Formats `source` parsed as `T` at the left margin.
    pub(in crate::pretty::syn) fn format_with_registry<T>(
        registry: &Registry,
        source: &str,
    ) -> String
    where
        T: Parse + PrettyPrint,
    {
        let ast: T = syn::parse_str(source).expect("test source must parse");
        let trivia: Vec<_> = Lexer::new(source).collect();
        let mut printer = Printer::new(registry, &trivia, MARGIN, 0);
        ast.pretty_print(&mut printer);
        printer.eof()
    }
}
