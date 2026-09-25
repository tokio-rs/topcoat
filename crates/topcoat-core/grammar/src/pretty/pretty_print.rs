use crate::pretty::Printer;

/// Writes a value as formatted text through a [`Printer`].
///
/// Implementations describe the layout. The printer chooses line breaks and
/// indentation.
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

impl<T> PrettyPrint for Box<T>
where
    T: PrettyPrint,
{
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.as_ref().pretty_print(printer);
    }
}
