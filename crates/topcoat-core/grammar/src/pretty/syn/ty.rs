use syn::spanned::Spanned;

use super::{common, path::qpath};
use crate::pretty::{BreakMode, Delim, PrettyPrint, Printer, TextMode};

impl PrettyPrint for syn::Type {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        match self {
            Self::Array(ty) => ty.pretty_print(printer),
            Self::BareFn(ty) => ty.pretty_print(printer),
            Self::Group(ty) => ty.elem.pretty_print(printer),
            Self::ImplTrait(ty) => ty.pretty_print(printer),
            Self::Infer(ty) => ty.underscore_token.pretty_print(printer),
            Self::Macro(ty) => ty.mac.pretty_print(printer),
            Self::Never(ty) => ty.bang_token.pretty_print(printer),
            Self::Paren(ty) => ty.pretty_print(printer),
            Self::Path(ty) => ty.pretty_print(printer),
            Self::Ptr(ty) => ty.pretty_print(printer),
            Self::Reference(ty) => ty.pretty_print(printer),
            Self::Slice(ty) => ty.pretty_print(printer),
            Self::TraitObject(ty) => ty.pretty_print(printer),
            Self::Tuple(ty) => ty.pretty_print(printer),
            Self::Verbatim(tokens) => {
                common::verbatim_span(printer, tokens.span(), || tokens.to_string());
            }
            _ => common::verbatim(printer, self),
        }
    }
}

impl PrettyPrint for syn::TypePath {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        qpath(printer, self.qself.as_ref(), &self.path);
    }
}

impl PrettyPrint for syn::TypeArray {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.bracket_token
            .pretty_print(printer, Some(BreakMode::Inconsistent), |printer| {
                self.elem.pretty_print(printer);
                self.semi_token.pretty_print(printer);
                " ".pretty_print(printer);
                self.len.pretty_print(printer);
            });
    }
}

impl PrettyPrint for syn::TypeSlice {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.bracket_token
            .pretty_print(printer, Some(BreakMode::Inconsistent), |printer| {
                self.elem.pretty_print(printer);
            });
    }
}

impl PrettyPrint for syn::TypeParen {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.paren_token
            .pretty_print(printer, Some(BreakMode::Inconsistent), |printer| {
                self.elem.pretty_print(printer);
            });
    }
}

impl PrettyPrint for syn::TypeTuple {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.paren_token
            .pretty_print(printer, Some(BreakMode::Consistent), |printer| {
                if self.elems.len() == 1 {
                    self.elems[0].pretty_print(printer);
                    printer.scan_text(",".into(), TextMode::Always);
                    printer.advance_cursor(",");
                } else {
                    self.elems.pretty_print(printer);
                }
            });
    }
}

impl PrettyPrint for syn::TypeReference {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.and_token.pretty_print(printer);
        if let Some(lifetime) = &self.lifetime {
            lifetime.pretty_print(printer);
            " ".pretty_print(printer);
        }
        if let Some(mutability) = &self.mutability {
            mutability.pretty_print(printer);
            " ".pretty_print(printer);
        }
        self.elem.pretty_print(printer);
    }
}

impl PrettyPrint for syn::TypePtr {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.star_token.pretty_print(printer);
        if let Some(const_token) = &self.const_token {
            const_token.pretty_print(printer);
            " ".pretty_print(printer);
        }
        if let Some(mutability) = &self.mutability {
            mutability.pretty_print(printer);
            " ".pretty_print(printer);
        }
        self.elem.pretty_print(printer);
    }
}

impl PrettyPrint for syn::TypeImplTrait {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.impl_token.pretty_print(printer);
        " ".pretty_print(printer);
        bounds(printer, &self.bounds);
    }
}

impl PrettyPrint for syn::TypeTraitObject {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        if let Some(dyn_token) = &self.dyn_token {
            dyn_token.pretty_print(printer);
            " ".pretty_print(printer);
        }
        bounds(printer, &self.bounds);
    }
}

impl PrettyPrint for syn::TypeBareFn {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.lifetimes.pretty_print(printer);
        if let Some(unsafety) = &self.unsafety {
            unsafety.pretty_print(printer);
            " ".pretty_print(printer);
        }
        if let Some(abi) = &self.abi {
            abi.pretty_print(printer);
            " ".pretty_print(printer);
        }
        self.fn_token.pretty_print(printer);
        self.paren_token
            .pretty_print(printer, Some(BreakMode::Consistent), |printer| {
                common::comma_separated(
                    printer,
                    &self.inputs,
                    self.variadic.as_ref().map(|variadic| variadic as _),
                );
            });
        self.output.pretty_print(printer);
    }
}

impl PrettyPrint for syn::BareFnArg {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        if let Some((name, colon)) = &self.name {
            name.pretty_print(printer);
            colon.pretty_print(printer);
            " ".pretty_print(printer);
        }
        self.ty.pretty_print(printer);
    }
}

impl PrettyPrint for syn::BareVariadic {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        if let Some((name, colon)) = &self.name {
            name.pretty_print(printer);
            colon.pretty_print(printer);
            " ".pretty_print(printer);
        }
        self.dots.pretty_print(printer);
    }
}

impl PrettyPrint for syn::Abi {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.extern_token.pretty_print(printer);
        if let Some(name) = &self.name {
            " ".pretty_print(printer);
            name.pretty_print(printer);
        }
    }
}

impl PrettyPrint for syn::ReturnType {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        if let Self::Type(arrow, ty) = self {
            " ".pretty_print(printer);
            arrow.pretty_print(printer);
            " ".pretty_print(printer);
            ty.pretty_print(printer);
        }
    }
}

impl PrettyPrint for syn::TypeParamBound {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        match self {
            Self::Trait(bound) => bound.pretty_print(printer),
            Self::Lifetime(lifetime) => lifetime.pretty_print(printer),
            _ => common::verbatim(printer, self),
        }
    }
}

impl PrettyPrint for syn::TraitBound {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        let inner = |printer: &mut Printer<'_>| {
            if let syn::TraitBoundModifier::Maybe(question) = &self.modifier {
                question.pretty_print(printer);
            }
            self.lifetimes.pretty_print(printer);
            self.path.pretty_print(printer);
        };
        match &self.paren_token {
            Some(paren) => paren.pretty_print(printer, Some(BreakMode::Inconsistent), inner),
            None => inner(printer),
        }
    }
}

impl PrettyPrint for syn::BoundLifetimes {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.for_token.pretty_print(printer);
        self.lt_token.pretty_print(printer);
        self.lifetimes.pretty_print(printer);
        self.gt_token.pretty_print(printer);
        " ".pretty_print(printer);
    }
}

/// Prints a `+`-separated bound list with a break point before each `+`.
pub(super) fn bounds(
    printer: &mut Printer<'_>,
    bounds: &syn::punctuated::Punctuated<syn::TypeParamBound, syn::Token![+]>,
) {
    printer.scan_begin(BreakMode::Inconsistent);
    printer.scan_indent(1);
    common::space_separated(printer, bounds);
    printer.scan_indent(-1);
    printer.scan_end();
}

#[cfg(test)]
mod tests {
    use super::super::common::tests::format;

    fn ty(source: &str) -> String {
        format::<syn::Type>(source)
    }

    #[test]
    fn path_type() {
        assert_eq!(ty("Vec<String>"), "Vec<String>");
    }

    #[test]
    fn reference() {
        assert_eq!(ty("&T"), "&T");
        assert_eq!(ty("&mut T"), "&mut T");
        assert_eq!(ty("&'a mut [u8]"), "&'a mut [u8]");
        assert_eq!(ty("&&str"), "&&str");
    }

    #[test]
    fn pointer() {
        assert_eq!(ty("*const u8"), "*const u8");
        assert_eq!(ty("*mut u8"), "*mut u8");
    }

    #[test]
    fn array_and_slice() {
        assert_eq!(ty("[u8; 4]"), "[u8; 4]");
        assert_eq!(ty("[String]"), "[String]");
    }

    #[test]
    fn tuples() {
        assert_eq!(ty("()"), "()");
        assert_eq!(ty("(A, B)"), "(A, B)");
        assert_eq!(ty("(A,)"), "(A,)");
    }

    #[test]
    fn bare_fn() {
        assert_eq!(ty("fn(usize) -> bool"), "fn(usize) -> bool");
        assert_eq!(
            ty("unsafe extern \"C\" fn(count: usize, ...)"),
            "unsafe extern \"C\" fn(count: usize, ...)",
        );
        assert_eq!(ty("for<'a> fn(&'a str)"), "for<'a> fn(&'a str)");
    }

    #[test]
    fn trait_object() {
        assert_eq!(
            ty("dyn Fn(u32) -> bool + Send + 'static"),
            "dyn Fn(u32) -> bool + Send + 'static",
        );
        assert_eq!(ty("Box<dyn Error + Send>"), "Box<dyn Error + Send>");
    }

    #[test]
    fn impl_trait() {
        assert_eq!(
            ty("impl Iterator<Item = u32> + Clone"),
            "impl Iterator<Item = u32> + Clone",
        );
    }

    #[test]
    fn maybe_sized_bound() {
        assert_eq!(ty("dyn Debug + ?Sized"), "dyn Debug + ?Sized");
        assert_eq!(ty("&(dyn Any + Send)"), "&(dyn Any + Send)");
    }

    #[test]
    fn never_and_infer() {
        assert_eq!(ty("!"), "!");
        assert_eq!(ty("_"), "_");
    }

    #[test]
    fn qualified_path() {
        assert_eq!(
            ty("<Vec<T> as IntoIterator>::IntoIter"),
            "<Vec<T> as IntoIterator>::IntoIter",
        );
    }

    #[test]
    fn macro_type() {
        assert_eq!(ty("ty!(u8)"), "ty!(u8)");
    }

    #[test]
    fn normalizes_spacing() {
        assert_eq!(ty("Vec < String >"), "Vec<String>");
        assert_eq!(ty("& mut T"), "&mut T");
        assert_eq!(ty("( A , B )"), "(A, B)");
    }

    #[test]
    fn long_bounds_break() {
        assert_eq!(
            ty(
                "impl ExtremelyLongTraitNameNumberOne + ExtremelyLongTraitNameNumberTwo + ExtremelyLongTraitNameNumberThree"
            ),
            "impl ExtremelyLongTraitNameNumberOne + ExtremelyLongTraitNameNumberTwo\n    + ExtremelyLongTraitNameNumberThree",
        );
    }
}
