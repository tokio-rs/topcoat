use crate::pretty::{Printer, TextMode};

/// Implemented by anything that knows how to emit itself as formatted text
/// through a [`Printer`]. The printer takes care of line breaking and
/// indentation; implementors only describe the desired layout.
pub trait PrettyPrint {
    fn pretty_print(&self, printer: &mut Printer<'_>);
}

impl<T> PrettyPrint for Option<T>
where
    T: PrettyPrint,
{
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        if let Some(inner) = self {
            inner.pretty_print(printer);
        }
    }
}

impl<T> PrettyPrint for [T]
where
    T: PrettyPrint,
{
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        for item in self {
            item.pretty_print(printer);
        }
    }
}

impl<T> PrettyPrint for syn::punctuated::Punctuated<T, syn::Token![,]>
where
    T: PrettyPrint,
{
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        for (index, item) in self.pairs().enumerate() {
            item.value().pretty_print(printer);
            if item.punct().is_some() {
                printer.scan_no_break_trivia();
            }
            if index == self.len() - 1 {
                printer.scan_text(",".into(), TextMode::Break);
                printer.advance_cursor(",");
            } else {
                item.punct().unwrap().pretty_print(printer);
                printer.scan_same_line_trivia();
                printer.scan_break();
                " ".pretty_print(printer);
                printer.scan_trivia(true, true);
            }
        }
    }
}
