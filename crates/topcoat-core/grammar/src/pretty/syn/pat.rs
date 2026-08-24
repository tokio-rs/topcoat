use syn::spanned::Spanned;

use super::{common, path::qpath};
use crate::pretty::{BreakMode, Delim, PrettyPrint, Printer, TextMode};

impl PrettyPrint for syn::Pat {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        match self {
            Self::Const(pat) => pat.pretty_print(printer),
            Self::Ident(pat) => pat.pretty_print(printer),
            Self::Lit(pat) => pat.pretty_print(printer),
            Self::Macro(pat) => pat.mac.pretty_print(printer),
            Self::Or(pat) => pat.pretty_print(printer),
            Self::Paren(pat) => pat.pretty_print(printer),
            Self::Path(pat) => pat.pretty_print(printer),
            Self::Range(pat) => pat.pretty_print(printer),
            Self::Reference(pat) => pat.pretty_print(printer),
            Self::Rest(pat) => pat.pretty_print(printer),
            Self::Slice(pat) => pat.pretty_print(printer),
            Self::Struct(pat) => pat.pretty_print(printer),
            Self::Tuple(pat) => pat.pretty_print(printer),
            Self::TupleStruct(pat) => pat.pretty_print(printer),
            Self::Type(pat) => pat.pretty_print(printer),
            Self::Verbatim(tokens) => {
                common::verbatim_span(printer, tokens.span(), || tokens.to_string());
            }
            Self::Wild(pat) => pat.pretty_print(printer),
            _ => common::verbatim(printer, self),
        }
    }
}

impl PrettyPrint for syn::PatType {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.pat.pretty_print(printer);
        self.colon_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.ty.pretty_print(printer);
    }
}

impl PrettyPrint for syn::PatIdent {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        if let Some(by_ref) = &self.by_ref {
            by_ref.pretty_print(printer);
            " ".pretty_print(printer);
        }
        if let Some(mutability) = &self.mutability {
            mutability.pretty_print(printer);
            " ".pretty_print(printer);
        }
        self.ident.pretty_print(printer);
        if let Some((at_token, subpat)) = &self.subpat {
            " ".pretty_print(printer);
            at_token.pretty_print(printer);
            " ".pretty_print(printer);
            subpat.pretty_print(printer);
        }
    }
}

impl PrettyPrint for syn::PatOr {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        if let Some(leading_vert) = &self.leading_vert {
            leading_vert.pretty_print(printer);
            " ".pretty_print(printer);
        }
        printer.scan_begin(BreakMode::Inconsistent);
        common::space_separated(printer, &self.cases);
        printer.scan_end();
    }
}

impl PrettyPrint for syn::PatParen {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.paren_token
            .pretty_print(printer, Some(BreakMode::Inconsistent), |printer| {
                self.pat.pretty_print(printer);
            });
    }
}

impl PrettyPrint for syn::PatReference {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.and_token.pretty_print(printer);
        if let Some(mutability) = &self.mutability {
            mutability.pretty_print(printer);
            " ".pretty_print(printer);
        }
        self.pat.pretty_print(printer);
    }
}

impl PrettyPrint for syn::PatRest {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.dot2_token.pretty_print(printer);
    }
}

impl PrettyPrint for syn::PatSlice {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.bracket_token
            .pretty_print(printer, Some(BreakMode::Consistent), |printer| {
                self.elems.pretty_print(printer);
            });
    }
}

impl PrettyPrint for syn::PatStruct {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        qpath(printer, self.qself.as_ref(), &self.path);
        " ".pretty_print(printer);
        if self.fields.is_empty()
            && self.rest.is_none()
            && !printer.has_comment_before(self.brace_token.span.close().start())
        {
            common::empty_braces(printer, &self.brace_token);
            return;
        }
        self.brace_token
            .pretty_print(printer, Some(BreakMode::Consistent), |printer| {
                common::comma_separated(
                    printer,
                    &self.fields,
                    self.rest.as_ref().map(|rest| rest as _),
                );
            });
    }
}

impl PrettyPrint for syn::FieldPat {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        if let Some(colon_token) = &self.colon_token {
            self.member.pretty_print(printer);
            colon_token.pretty_print(printer);
            " ".pretty_print(printer);
        }
        self.pat.pretty_print(printer);
    }
}

impl PrettyPrint for syn::PatTuple {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.paren_token
            .pretty_print(printer, Some(BreakMode::Consistent), |printer| {
                if self.elems.len() == 1 && !matches!(self.elems[0], syn::Pat::Rest(_)) {
                    self.elems[0].pretty_print(printer);
                    printer.scan_text(",".into(), TextMode::Always);
                    printer.advance_cursor(",");
                } else {
                    self.elems.pretty_print(printer);
                }
            });
    }
}

impl PrettyPrint for syn::PatTupleStruct {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        qpath(printer, self.qself.as_ref(), &self.path);
        self.paren_token
            .pretty_print(printer, Some(BreakMode::Consistent), |printer| {
                self.elems.pretty_print(printer);
            });
    }
}

impl PrettyPrint for syn::PatWild {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.underscore_token.pretty_print(printer);
    }
}

#[cfg(test)]
mod tests {
    use syn::parse::{Parse, ParseStream};

    use super::super::common::tests::format;
    use crate::pretty::{PrettyPrint, Printer};

    /// Wraps [`syn::Pat`], which offers only inherent parse methods, so the
    /// test helper can parse one through [`Parse`].
    struct TestPat(syn::Pat);

    impl Parse for TestPat {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            Ok(Self(syn::Pat::parse_multi_with_leading_vert(input)?))
        }
    }

    impl PrettyPrint for TestPat {
        fn pretty_print(&self, printer: &mut Printer<'_>) {
            self.0.pretty_print(printer);
        }
    }

    fn pat(source: &str) -> String {
        format::<TestPat>(source)
    }

    #[test]
    fn binding_modes() {
        assert_eq!(pat("x"), "x");
        assert_eq!(pat("mut x"), "mut x");
        assert_eq!(pat("ref x"), "ref x");
        assert_eq!(pat("ref mut x"), "ref mut x");
    }

    #[test]
    fn wildcard_and_rest() {
        assert_eq!(pat("_"), "_");
        assert_eq!(pat("(a, ..)"), "(a, ..)");
    }

    #[test]
    fn subpattern_binding() {
        assert_eq!(pat("first @ Some(_)"), "first @ Some(_)");
        assert_eq!(pat("[head, tail @ ..]"), "[head, tail @ ..]");
    }

    #[test]
    fn tuple_struct() {
        assert_eq!(pat("Some(value)"), "Some(value)");
        assert_eq!(pat("Point(x, y)"), "Point(x, y)");
    }

    #[test]
    fn struct_pattern() {
        assert_eq!(pat("Point { x, y }"), "Point { x, y }");
        assert_eq!(pat("Point { x: 0, .. }"), "Point { x: 0, .. }");
        assert_eq!(pat("Person { name, age: 33 }"), "Person { name, age: 33 }");
    }

    #[test]
    fn empty_struct_pattern() {
        assert_eq!(pat("Empty {}"), "Empty {}");
    }

    #[test]
    fn tuples() {
        assert_eq!(pat("(a, b)"), "(a, b)");
        assert_eq!(pat("(a,)"), "(a,)");
    }

    #[test]
    fn slices() {
        assert_eq!(pat("[]"), "[]");
        assert_eq!(pat("[first, .., last]"), "[first, .., last]");
    }

    #[test]
    fn references() {
        assert_eq!(pat("&value"), "&value");
        assert_eq!(pat("&mut value"), "&mut value");
    }

    #[test]
    fn or_pattern() {
        assert_eq!(pat("A | B | C"), "A | B | C");
        assert_eq!(pat("| A | B"), "| A | B");
        assert_eq!(pat("Some(1 | 2)"), "Some(1 | 2)");
    }

    #[test]
    fn range_patterns() {
        assert_eq!(pat("1..=9"), "1..=9");
        assert_eq!(pat("'a'..='z'"), "'a'..='z'");
    }

    #[test]
    fn literals_and_paths() {
        assert_eq!(pat("42"), "42");
        assert_eq!(pat("Status::Active"), "Status::Active");
    }

    #[test]
    fn normalizes_spacing() {
        assert_eq!(pat("Point{x , y}"), "Point { x, y }");
        assert_eq!(pat("Some( x )"), "Some(x)");
    }

    #[test]
    fn long_struct_pattern_breaks() {
        assert_eq!(
            pat(
                "Configuration { first_extremely_long_field_name, second_extremely_long_field_name, third, .. }"
            ),
            "Configuration {\n    first_extremely_long_field_name,\n    second_extremely_long_field_name,\n    third,\n    ..\n}",
        );
    }

    #[test]
    fn comment_in_struct_pattern() {
        assert_eq!(
            pat("Point { x, /* height */ y }"),
            "Point { x, /* height */ y }",
        );
    }
}
