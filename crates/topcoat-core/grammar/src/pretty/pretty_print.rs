use crate::pretty::Printer;

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

impl<T> PrettyPrint for Box<T>
where
    T: PrettyPrint,
{
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.as_ref().pretty_print(printer);
    }
}
