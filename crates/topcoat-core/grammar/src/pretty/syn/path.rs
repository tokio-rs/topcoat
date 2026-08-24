use super::common;
use crate::pretty::{BreakMode, Delim, PrettyPrint, Printer};

impl PrettyPrint for syn::Path {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.leading_colon.pretty_print(printer);
        for pair in self.segments.pairs() {
            pair.value().pretty_print(printer);
            if let Some(punct) = pair.punct() {
                punct.pretty_print(printer);
            }
        }
    }
}

impl PrettyPrint for syn::PathSegment {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.ident.pretty_print(printer);
        self.arguments.pretty_print(printer);
    }
}

impl PrettyPrint for syn::PathArguments {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        match self {
            Self::None => {}
            Self::AngleBracketed(arguments) => arguments.pretty_print(printer),
            Self::Parenthesized(arguments) => arguments.pretty_print(printer),
        }
    }
}

/// Prints a possibly qualified path: a plain `a::b`, a `<T as Trait>::item`, or
/// a `<T>::item`.
pub(super) fn qpath(printer: &mut Printer<'_>, qself: Option<&syn::QSelf>, path: &syn::Path) {
    let Some(qself) = qself else {
        path.pretty_print(printer);
        return;
    };

    qself.lt_token.pretty_print(printer);
    qself.ty.pretty_print(printer);
    if let Some(as_token) = &qself.as_token {
        " ".pretty_print(printer);
        as_token.pretty_print(printer);
        " ".pretty_print(printer);
        path.leading_colon.pretty_print(printer);
    }

    let pairs: Vec<_> = path.segments.pairs().collect();
    for (index, pair) in pairs.iter().enumerate() {
        if index == qself.position {
            qself.gt_token.pretty_print(printer);
        }
        if index > 0 {
            match pairs[index - 1].punct() {
                Some(punct) => punct.pretty_print(printer),
                None => "::".pretty_print(printer),
            }
        } else if qself.position == 0 {
            match &path.leading_colon {
                Some(colon) => colon.pretty_print(printer),
                None => "::".pretty_print(printer),
            }
        }
        pair.value().pretty_print(printer);
    }
}

impl PrettyPrint for syn::AngleBracketedGenericArguments {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.colon2_token.pretty_print(printer);
        self.lt_token.pretty_print(printer);
        printer.scan_begin(BreakMode::Consistent);
        printer.scan_indent(1);
        printer.scan_break();
        self.args.pretty_print(printer);
        printer.scan_indent(-1);
        printer.scan_break();
        printer.scan_end();
        self.gt_token.pretty_print(printer);
    }
}

impl PrettyPrint for syn::GenericArgument {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        match self {
            Self::Lifetime(lifetime) => lifetime.pretty_print(printer),
            Self::Type(ty) => ty.pretty_print(printer),
            Self::Const(expr) => const_argument(printer, expr),
            Self::AssocType(assoc) => assoc.pretty_print(printer),
            Self::AssocConst(assoc) => assoc.pretty_print(printer),
            Self::Constraint(constraint) => constraint.pretty_print(printer),
            _ => common::verbatim(printer, self),
        }
    }
}

/// Prints a const generic argument. A braced argument (`{ N + 1 }`) stays on
/// one line rather than breaking like a block would.
fn const_argument(printer: &mut Printer<'_>, expr: &syn::Expr) {
    if let syn::Expr::Block(block) = expr
        && block.attrs.is_empty()
        && block.label.is_none()
        && let [stmt] = block.block.stmts.as_slice()
    {
        let brace = &block.block.brace_token;
        common::token(printer, "{", brace.span.open());
        " ".pretty_print(printer);
        stmt.pretty_print(printer);
        printer.move_cursor(brace.span.close().start());
        printer.scan_no_break_trivia();
        " ".pretty_print(printer);
        common::token(printer, "}", brace.span.close());
        return;
    }
    expr.pretty_print(printer);
}

impl PrettyPrint for syn::AssocType {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.ident.pretty_print(printer);
        self.generics.pretty_print(printer);
        " ".pretty_print(printer);
        self.eq_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.ty.pretty_print(printer);
    }
}

impl PrettyPrint for syn::AssocConst {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.ident.pretty_print(printer);
        self.generics.pretty_print(printer);
        " ".pretty_print(printer);
        self.eq_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.value.pretty_print(printer);
    }
}

impl PrettyPrint for syn::Constraint {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.ident.pretty_print(printer);
        self.generics.pretty_print(printer);
        self.colon_token.pretty_print(printer);
        " ".pretty_print(printer);
        printer.scan_begin(BreakMode::Inconsistent);
        printer.scan_indent(1);
        common::space_separated(printer, &self.bounds);
        printer.scan_indent(-1);
        printer.scan_end();
    }
}

impl PrettyPrint for syn::ParenthesizedGenericArguments {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.paren_token
            .pretty_print(printer, Some(BreakMode::Consistent), |printer| {
                self.inputs.pretty_print(printer);
            });
        self.output.pretty_print(printer);
    }
}

#[cfg(test)]
mod tests {
    use super::super::common::tests::format;

    fn path(source: &str) -> String {
        format::<syn::Path>(source)
    }

    #[test]
    fn plain_path() {
        assert_eq!(
            path("std::collections::HashMap"),
            "std::collections::HashMap"
        );
    }

    #[test]
    fn leading_colon_path() {
        assert_eq!(path("::std::mem::drop"), "::std::mem::drop");
    }

    #[test]
    fn generic_arguments() {
        assert_eq!(path("HashMap<String, u32>"), "HashMap<String, u32>");
    }

    #[test]
    fn nested_generic_arguments() {
        assert_eq!(
            path("Vec<HashMap<String, Vec<u8>>>"),
            "Vec<HashMap<String, Vec<u8>>>",
        );
    }

    #[test]
    fn turbofish() {
        assert_eq!(path("Vec::<u8>::new"), "Vec::<u8>::new");
    }

    #[test]
    fn lifetime_argument() {
        assert_eq!(path("Cow<'a, str>"), "Cow<'a, str>");
    }

    #[test]
    fn associated_type_binding() {
        assert_eq!(
            path("Iterator<Item = (String, u32)>"),
            "Iterator<Item = (String, u32)>",
        );
    }

    #[test]
    fn constraint_argument() {
        assert_eq!(
            path("Iterator<Item: Clone + Send>"),
            "Iterator<Item: Clone + Send>",
        );
    }

    #[test]
    fn const_argument() {
        assert_eq!(path("Array<u8, 4>"), "Array<u8, 4>");
    }

    #[test]
    fn braced_const_argument_stays_inline() {
        assert_eq!(path("Array<u8, { N + 1 }>"), "Array<u8, { N + 1 }>");
    }

    #[test]
    fn parenthesized_arguments() {
        assert_eq!(
            format::<syn::Type>("dyn Fn(u32, String) -> bool"),
            "dyn Fn(u32, String) -> bool",
        );
    }

    #[test]
    fn long_generic_arguments_break() {
        assert_eq!(
            path(
                "HashMap<SomeExtremelyLongKeyTypeName, AnotherExtremelyLongValueTypeName, DefaultHasherState>"
            ),
            "HashMap<\n    SomeExtremelyLongKeyTypeName,\n    AnotherExtremelyLongValueTypeName,\n    DefaultHasherState,\n>",
        );
    }

    #[test]
    fn qualified_path_with_trait() {
        assert_eq!(
            format::<syn::TypePath>("<Vec<T> as IntoIterator>::Item"),
            "<Vec<T> as IntoIterator>::Item",
        );
    }

    #[test]
    fn qualified_path_without_trait() {
        assert_eq!(format::<syn::TypePath>("<Vec<T>>::Item"), "<Vec<T>>::Item");
    }

    #[test]
    fn comment_between_generic_arguments() {
        assert_eq!(
            path("HashMap<String, /* value */ u32>"),
            "HashMap<String, /* value */ u32>",
        );
    }
}
