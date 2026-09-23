use crate::pretty::Printer;

/// A syntax node that can print itself as formatted text through a
/// [`Printer`].
///
/// An implementation describes the layout by feeding text, breaks, and groups
/// to the printer. The printer then decides which breaks become line breaks
/// and handles indentation.
pub trait PrettyPrint {
    /// Feeds this node to `printer`.
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

impl<T> PrettyPrint for Box<T>
where
    T: PrettyPrint,
{
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.as_ref().pretty_print(printer);
    }
}
