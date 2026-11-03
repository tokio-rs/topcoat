use proc_macro2::Literal;

use super::{PrettyPrint, Printer, TextMode};

impl PrettyPrint for &'static str {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        printer.scan_text((*self).into(), TextMode::Always);
        printer.advance_cursor(self);
    }
}

impl PrettyPrint for String {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        printer.scan_text(self.clone().into(), TextMode::Always);
        printer.advance_cursor(self);
    }
}

impl PrettyPrint for Literal {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        printer.move_cursor(self.span().start());
        printer.scan_text(self.to_string().into(), TextMode::Always);
        printer.move_cursor(self.span().end());
    }
}
