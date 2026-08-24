use syn::spanned::Spanned;

use super::{common, ty::bounds};
use crate::pretty::{BreakMode, Delim, PrettyPrint, Printer, Unspaced};

impl PrettyPrint for syn::Item {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        match self {
            Self::Const(item) => item.pretty_print(printer),
            Self::Enum(item) => item.pretty_print(printer),
            Self::Fn(item) => item.pretty_print(printer),
            Self::Impl(item) => item.pretty_print(printer),
            Self::Macro(item) => item.pretty_print(printer),
            Self::Mod(item) => item.pretty_print(printer),
            Self::Static(item) => item.pretty_print(printer),
            Self::Struct(item) => item.pretty_print(printer),
            Self::Trait(item) => item.pretty_print(printer),
            Self::Type(item) => item.pretty_print(printer),
            Self::Use(item) => item.pretty_print(printer),
            _ => common::verbatim(printer, self),
        }
    }
}

impl PrettyPrint for syn::Visibility {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        match self {
            Self::Public(pub_token) => {
                pub_token.pretty_print(printer);
                " ".pretty_print(printer);
            }
            Self::Restricted(restricted) => {
                restricted.pub_token.pretty_print(printer);
                common::token(printer, "(", restricted.paren_token.span.open());
                if let Some(in_token) = &restricted.in_token {
                    in_token.pretty_print(printer);
                    " ".pretty_print(printer);
                }
                restricted.path.pretty_print(printer);
                common::token(printer, ")", restricted.paren_token.span.close());
                " ".pretty_print(printer);
            }
            Self::Inherited => {}
        }
    }
}

impl PrettyPrint for syn::Generics {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        if self.params.is_empty() {
            return;
        }
        let (Some(lt_token), Some(gt_token)) = (&self.lt_token, &self.gt_token) else {
            return;
        };
        lt_token.pretty_print(printer);
        printer.scan_begin(BreakMode::Consistent);
        printer.scan_indent(1);
        printer.scan_break();
        self.params.pretty_print(printer);
        printer.scan_indent(-1);
        printer.scan_break();
        printer.scan_end();
        gt_token.pretty_print(printer);
    }
}

impl PrettyPrint for syn::GenericParam {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        match self {
            Self::Lifetime(param) => param.pretty_print(printer),
            Self::Type(param) => param.pretty_print(printer),
            Self::Const(param) => param.pretty_print(printer),
        }
    }
}

impl PrettyPrint for syn::LifetimeParam {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.lifetime.pretty_print(printer);
        if let Some(colon_token) = &self.colon_token {
            colon_token.pretty_print(printer);
            " ".pretty_print(printer);
            printer.scan_begin(BreakMode::Inconsistent);
            common::space_separated(printer, &self.bounds);
            printer.scan_end();
        }
    }
}

impl PrettyPrint for syn::TypeParam {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.ident.pretty_print(printer);
        if let Some(colon_token) = &self.colon_token {
            colon_token.pretty_print(printer);
            " ".pretty_print(printer);
            bounds(printer, &self.bounds);
        }
        if let Some(default) = &self.default {
            " ".pretty_print(printer);
            self.eq_token.pretty_print(printer);
            " ".pretty_print(printer);
            default.pretty_print(printer);
        }
    }
}

impl PrettyPrint for syn::ConstParam {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.const_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.ident.pretty_print(printer);
        self.colon_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.ty.pretty_print(printer);
        if let Some(default) = &self.default {
            " ".pretty_print(printer);
            self.eq_token.pretty_print(printer);
            " ".pretty_print(printer);
            default.pretty_print(printer);
        }
    }
}

impl PrettyPrint for syn::WherePredicate {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        match self {
            Self::Lifetime(predicate) => {
                predicate.lifetime.pretty_print(printer);
                predicate.colon_token.pretty_print(printer);
                " ".pretty_print(printer);
                printer.scan_begin(BreakMode::Inconsistent);
                common::space_separated(printer, &predicate.bounds);
                printer.scan_end();
            }
            Self::Type(predicate) => {
                predicate.lifetimes.pretty_print(printer);
                predicate.bounded_ty.pretty_print(printer);
                predicate.colon_token.pretty_print(printer);
                " ".pretty_print(printer);
                bounds(printer, &predicate.bounds);
            }
            _ => common::verbatim(printer, self),
        }
    }
}

/// Prints a `where` clause on its own line with one predicate per line. When
/// `terminated`, the last predicate carries no comma so a `;` can follow it
/// directly; otherwise the clause ends with a break for the body that follows.
fn print_where(printer: &mut Printer<'_>, clause: &syn::WhereClause, terminated: bool) {
    printer.scan_begin(BreakMode::Consistent);
    printer.scan_force_break();
    printer.scan_break();
    clause.where_token.pretty_print(printer);
    printer.scan_indent(1);
    for (index, pair) in clause.predicates.pairs().enumerate() {
        printer.scan_force_break();
        printer.scan_break();
        pair.value().pretty_print(printer);
        if !(terminated && index == clause.predicates.len() - 1) {
            common::comma(printer, pair.punct().copied());
        }
    }
    printer.scan_indent(-1);
    if !terminated {
        printer.scan_force_break();
        printer.scan_break();
    }
    printer.scan_end();
}

/// Prints the separation between an item's generics and its braced body: a
/// broken `where` clause with the body on the following line, or a single
/// space.
fn where_clause_or_space(printer: &mut Printer<'_>, generics: &syn::Generics) {
    match &generics.where_clause {
        Some(clause) => print_where(printer, clause, false),
        None => " ".pretty_print(printer),
    }
}

impl PrettyPrint for syn::Signature {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        if let Some(constness) = &self.constness {
            constness.pretty_print(printer);
            " ".pretty_print(printer);
        }
        if let Some(asyncness) = &self.asyncness {
            asyncness.pretty_print(printer);
            " ".pretty_print(printer);
        }
        if let Some(unsafety) = &self.unsafety {
            unsafety.pretty_print(printer);
            " ".pretty_print(printer);
        }
        if let Some(abi) = &self.abi {
            abi.pretty_print(printer);
            " ".pretty_print(printer);
        }
        self.fn_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.ident.pretty_print(printer);
        self.generics.pretty_print(printer);
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

impl PrettyPrint for syn::FnArg {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        match self {
            Self::Receiver(receiver) => receiver.pretty_print(printer),
            Self::Typed(arg) => arg.pretty_print(printer),
        }
    }
}

impl PrettyPrint for syn::Receiver {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        if let Some(colon_token) = &self.colon_token {
            self.self_token.pretty_print(printer);
            colon_token.pretty_print(printer);
            " ".pretty_print(printer);
            self.ty.pretty_print(printer);
            return;
        }
        if let Some((and_token, lifetime)) = &self.reference {
            and_token.pretty_print(printer);
            if let Some(lifetime) = lifetime {
                lifetime.pretty_print(printer);
                " ".pretty_print(printer);
            }
        }
        if let Some(mutability) = &self.mutability {
            mutability.pretty_print(printer);
            " ".pretty_print(printer);
        }
        self.self_token.pretty_print(printer);
    }
}

impl PrettyPrint for syn::Variadic {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        if let Some((pat, colon_token)) = &self.pat {
            pat.pretty_print(printer);
            colon_token.pretty_print(printer);
            " ".pretty_print(printer);
        }
        self.dots.pretty_print(printer);
    }
}

impl PrettyPrint for syn::ItemFn {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.vis.pretty_print(printer);
        self.sig.pretty_print(printer);
        where_clause_or_space(printer, &self.sig.generics);
        self.block.pretty_print(printer);
    }
}

impl PrettyPrint for syn::ItemConst {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        if self.generics.where_clause.is_some() {
            common::verbatim(printer, self);
            return;
        }
        self.attrs.pretty_print(printer);
        self.vis.pretty_print(printer);
        self.const_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.ident.pretty_print(printer);
        self.generics.pretty_print(printer);
        self.colon_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.ty.pretty_print(printer);
        " ".pretty_print(printer);
        self.eq_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.expr.pretty_print(printer);
        self.semi_token.pretty_print(printer);
    }
}

impl PrettyPrint for syn::ItemStatic {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.vis.pretty_print(printer);
        self.static_token.pretty_print(printer);
        " ".pretty_print(printer);
        if let syn::StaticMutability::Mut(mut_token) = &self.mutability {
            mut_token.pretty_print(printer);
            " ".pretty_print(printer);
        }
        self.ident.pretty_print(printer);
        self.colon_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.ty.pretty_print(printer);
        " ".pretty_print(printer);
        self.eq_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.expr.pretty_print(printer);
        self.semi_token.pretty_print(printer);
    }
}

impl PrettyPrint for syn::ItemType {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        if self.generics.where_clause.is_some() {
            common::verbatim(printer, self);
            return;
        }
        self.attrs.pretty_print(printer);
        self.vis.pretty_print(printer);
        self.type_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.ident.pretty_print(printer);
        self.generics.pretty_print(printer);
        " ".pretty_print(printer);
        self.eq_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.ty.pretty_print(printer);
        self.semi_token.pretty_print(printer);
    }
}

impl PrettyPrint for syn::ItemUse {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.vis.pretty_print(printer);
        self.use_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.leading_colon.pretty_print(printer);
        self.tree.pretty_print(printer);
        self.semi_token.pretty_print(printer);
    }
}

impl PrettyPrint for syn::UseTree {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        match self {
            Self::Path(path) => {
                path.ident.pretty_print(printer);
                path.colon2_token.pretty_print(printer);
                path.tree.pretty_print(printer);
            }
            Self::Name(name) => name.ident.pretty_print(printer),
            Self::Rename(rename) => {
                rename.ident.pretty_print(printer);
                " ".pretty_print(printer);
                rename.as_token.pretty_print(printer);
                " ".pretty_print(printer);
                rename.rename.pretty_print(printer);
            }
            Self::Glob(glob) => glob.star_token.pretty_print(printer),
            Self::Group(group) => {
                Unspaced(&group.brace_token).pretty_print(
                    printer,
                    Some(BreakMode::Consistent),
                    |printer| {
                        group.items.pretty_print(printer);
                    },
                );
            }
        }
    }
}

impl PrettyPrint for syn::ItemStruct {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.vis.pretty_print(printer);
        self.struct_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.ident.pretty_print(printer);
        self.generics.pretty_print(printer);
        match &self.fields {
            syn::Fields::Named(fields) => {
                where_clause_or_space(printer, &self.generics);
                forced_named_fields(printer, fields);
            }
            syn::Fields::Unnamed(fields) => {
                fields.pretty_print(printer);
                if let Some(clause) = &self.generics.where_clause {
                    print_where(printer, clause, true);
                }
                self.semi_token.pretty_print(printer);
            }
            syn::Fields::Unit => {
                if let Some(clause) = &self.generics.where_clause {
                    print_where(printer, clause, true);
                }
                self.semi_token.pretty_print(printer);
            }
        }
    }
}

impl PrettyPrint for syn::ItemEnum {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.vis.pretty_print(printer);
        self.enum_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.ident.pretty_print(printer);
        self.generics.pretty_print(printer);
        where_clause_or_space(printer, &self.generics);
        common::forced_comma_braces(printer, &self.brace_token, &self.variants);
    }
}

impl PrettyPrint for syn::Variant {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.ident.pretty_print(printer);
        self.fields.pretty_print(printer);
        if let Some((eq_token, discriminant)) = &self.discriminant {
            " ".pretty_print(printer);
            eq_token.pretty_print(printer);
            " ".pretty_print(printer);
            discriminant.pretty_print(printer);
        }
    }
}

impl PrettyPrint for syn::Fields {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        match self {
            Self::Named(fields) => {
                " ".pretty_print(printer);
                fields.pretty_print(printer);
            }
            Self::Unnamed(fields) => fields.pretty_print(printer),
            Self::Unit => {}
        }
    }
}

impl PrettyPrint for syn::FieldsNamed {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.brace_token
            .pretty_print(printer, Some(BreakMode::Consistent), |printer| {
                self.named.pretty_print(printer);
            });
    }
}

impl PrettyPrint for syn::FieldsUnnamed {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.paren_token
            .pretty_print(printer, Some(BreakMode::Consistent), |printer| {
                self.unnamed.pretty_print(printer);
            });
    }
}

impl PrettyPrint for syn::Field {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.vis.pretty_print(printer);
        if let Some(ident) = &self.ident {
            ident.pretty_print(printer);
            self.colon_token.pretty_print(printer);
            " ".pretty_print(printer);
        }
        self.ty.pretty_print(printer);
    }
}

/// Prints a struct's named fields, always one per line.
fn forced_named_fields(printer: &mut Printer<'_>, fields: &syn::FieldsNamed) {
    common::forced_comma_braces(printer, &fields.brace_token, &fields.named);
}

impl PrettyPrint for syn::ItemMod {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.vis.pretty_print(printer);
        if let Some(unsafety) = &self.unsafety {
            unsafety.pretty_print(printer);
            " ".pretty_print(printer);
        }
        self.mod_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.ident.pretty_print(printer);
        match &self.content {
            Some((brace, items)) => {
                " ".pretty_print(printer);
                common::statement_braces(printer, brace, items);
            }
            None => self.semi.pretty_print(printer),
        }
    }
}

impl PrettyPrint for syn::ItemImpl {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        if let Some(defaultness) = &self.defaultness {
            defaultness.pretty_print(printer);
            " ".pretty_print(printer);
        }
        if let Some(unsafety) = &self.unsafety {
            unsafety.pretty_print(printer);
            " ".pretty_print(printer);
        }
        self.impl_token.pretty_print(printer);
        self.generics.pretty_print(printer);
        " ".pretty_print(printer);
        if let Some((not_token, path, for_token)) = &self.trait_ {
            not_token.pretty_print(printer);
            path.pretty_print(printer);
            " ".pretty_print(printer);
            for_token.pretty_print(printer);
            " ".pretty_print(printer);
        }
        self.self_ty.pretty_print(printer);
        where_clause_or_space(printer, &self.generics);
        common::statement_braces(printer, &self.brace_token, &self.items);
    }
}

impl PrettyPrint for syn::ImplItem {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        match self {
            Self::Const(item) => item.pretty_print(printer),
            Self::Fn(item) => item.pretty_print(printer),
            Self::Type(item) => item.pretty_print(printer),
            Self::Macro(item) => {
                item.attrs.pretty_print(printer);
                item.mac.pretty_print(printer);
                item.semi_token.pretty_print(printer);
            }
            _ => common::verbatim(printer, self),
        }
    }
}

impl PrettyPrint for syn::ImplItemFn {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.vis.pretty_print(printer);
        if let Some(defaultness) = &self.defaultness {
            defaultness.pretty_print(printer);
            " ".pretty_print(printer);
        }
        self.sig.pretty_print(printer);
        where_clause_or_space(printer, &self.sig.generics);
        self.block.pretty_print(printer);
    }
}

impl PrettyPrint for syn::ImplItemConst {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        if self.generics.where_clause.is_some() {
            common::verbatim(printer, self);
            return;
        }
        self.attrs.pretty_print(printer);
        self.vis.pretty_print(printer);
        if let Some(defaultness) = &self.defaultness {
            defaultness.pretty_print(printer);
            " ".pretty_print(printer);
        }
        self.const_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.ident.pretty_print(printer);
        self.generics.pretty_print(printer);
        self.colon_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.ty.pretty_print(printer);
        " ".pretty_print(printer);
        self.eq_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.expr.pretty_print(printer);
        self.semi_token.pretty_print(printer);
    }
}

impl PrettyPrint for syn::ImplItemType {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        if self.generics.where_clause.is_some() {
            common::verbatim(printer, self);
            return;
        }
        self.attrs.pretty_print(printer);
        self.vis.pretty_print(printer);
        if let Some(defaultness) = &self.defaultness {
            defaultness.pretty_print(printer);
            " ".pretty_print(printer);
        }
        self.type_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.ident.pretty_print(printer);
        self.generics.pretty_print(printer);
        " ".pretty_print(printer);
        self.eq_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.ty.pretty_print(printer);
        self.semi_token.pretty_print(printer);
    }
}

impl PrettyPrint for syn::ItemTrait {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.vis.pretty_print(printer);
        if let Some(unsafety) = &self.unsafety {
            unsafety.pretty_print(printer);
            " ".pretty_print(printer);
        }
        if let Some(auto_token) = &self.auto_token {
            auto_token.pretty_print(printer);
            " ".pretty_print(printer);
        }
        self.trait_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.ident.pretty_print(printer);
        self.generics.pretty_print(printer);
        if let Some(colon_token) = &self.colon_token {
            colon_token.pretty_print(printer);
            " ".pretty_print(printer);
            bounds(printer, &self.supertraits);
        }
        where_clause_or_space(printer, &self.generics);
        common::statement_braces(printer, &self.brace_token, &self.items);
    }
}

impl PrettyPrint for syn::TraitItem {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        match self {
            Self::Const(item) => item.pretty_print(printer),
            Self::Fn(item) => item.pretty_print(printer),
            Self::Type(item) => item.pretty_print(printer),
            Self::Macro(item) => {
                item.attrs.pretty_print(printer);
                item.mac.pretty_print(printer);
                item.semi_token.pretty_print(printer);
            }
            _ => common::verbatim(printer, self),
        }
    }
}

impl PrettyPrint for syn::TraitItemFn {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.sig.pretty_print(printer);
        if let Some(block) = &self.default {
            where_clause_or_space(printer, &self.sig.generics);
            block.pretty_print(printer);
        } else {
            if let Some(clause) = &self.sig.generics.where_clause {
                print_where(printer, clause, true);
            }
            self.semi_token.pretty_print(printer);
        }
    }
}

impl PrettyPrint for syn::TraitItemConst {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        if self.generics.where_clause.is_some() {
            common::verbatim(printer, self);
            return;
        }
        self.attrs.pretty_print(printer);
        self.const_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.ident.pretty_print(printer);
        self.generics.pretty_print(printer);
        self.colon_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.ty.pretty_print(printer);
        if let Some((eq_token, default)) = &self.default {
            " ".pretty_print(printer);
            eq_token.pretty_print(printer);
            " ".pretty_print(printer);
            default.pretty_print(printer);
        }
        self.semi_token.pretty_print(printer);
    }
}

impl PrettyPrint for syn::TraitItemType {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        if self.generics.where_clause.is_some() {
            common::verbatim(printer, self);
            return;
        }
        self.attrs.pretty_print(printer);
        self.type_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.ident.pretty_print(printer);
        self.generics.pretty_print(printer);
        if let Some(colon_token) = &self.colon_token {
            colon_token.pretty_print(printer);
            " ".pretty_print(printer);
            bounds(printer, &self.bounds);
        }
        if let Some((eq_token, default)) = &self.default {
            " ".pretty_print(printer);
            eq_token.pretty_print(printer);
            " ".pretty_print(printer);
            default.pretty_print(printer);
        }
        self.semi_token.pretty_print(printer);
    }
}

impl PrettyPrint for syn::ItemMacro {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        if let Some(ident) = &self.ident {
            // A `macro_rules! name { ... }` definition; the body is arbitrary
            // token trees and is copied verbatim.
            self.mac.path.pretty_print(printer);
            self.mac.bang_token.pretty_print(printer);
            " ".pretty_print(printer);
            ident.pretty_print(printer);
            " ".pretty_print(printer);
            let span = self.mac.delimiter.span().span();
            common::verbatim_span(printer, span, || format!("{{ {} }}", self.mac.tokens));
        } else {
            self.mac.pretty_print(printer);
        }
        self.semi_token.pretty_print(printer);
    }
}

#[cfg(test)]
mod tests {
    use super::super::common::tests::format;

    fn item(source: &str) -> String {
        format::<syn::Item>(source)
    }

    #[test]
    fn plain_function() {
        assert_eq!(
            item("fn add(a: u32, b: u32) -> u32 { a + b }"),
            "fn add(a: u32, b: u32) -> u32 {\n    a + b\n}",
        );
    }

    #[test]
    fn empty_function() {
        assert_eq!(item("fn noop() {}"), "fn noop() {}");
    }

    #[test]
    fn function_qualifiers() {
        assert_eq!(
            item("pub async unsafe fn go() {}"),
            "pub async unsafe fn go() {}",
        );
        assert_eq!(
            item("pub(crate) fn helper() {}"),
            "pub(crate) fn helper() {}"
        );
    }

    #[test]
    fn function_with_where_clause() {
        assert_eq!(
            item("fn largest<T>(list: &[T]) -> T where T: PartialOrd + Copy { todo!() }"),
            "fn largest<T>(list: &[T]) -> T\nwhere\n    T: PartialOrd + Copy,\n{\n    todo!()\n}",
        );
    }

    #[test]
    fn long_signature_breaks_arguments() {
        assert_eq!(
            item(
                "fn configure(first_extremely_long_parameter: FirstType, second_extremely_long_parameter: SecondType) {}"
            ),
            "fn configure(\n    first_extremely_long_parameter: FirstType,\n    second_extremely_long_parameter: SecondType,\n) {}",
        );
    }

    #[test]
    fn struct_fields_one_per_line() {
        assert_eq!(
            item("struct Point { x: f32, y: f32 }"),
            "struct Point {\n    x: f32,\n    y: f32,\n}",
        );
    }

    #[test]
    fn unit_and_tuple_structs() {
        assert_eq!(item("struct Marker;"), "struct Marker;");
        assert_eq!(item("struct Pair(u32, u32);"), "struct Pair(u32, u32);");
        assert_eq!(item("pub struct Id(pub u64);"), "pub struct Id(pub u64);");
    }

    #[test]
    fn empty_struct() {
        assert_eq!(item("struct Empty {}"), "struct Empty {}");
    }

    #[test]
    fn generic_struct() {
        assert_eq!(
            item("struct Wrapper<T: Clone, const N: usize> { items: [T; N] }"),
            "struct Wrapper<T: Clone, const N: usize> {\n    items: [T; N],\n}",
        );
    }

    #[test]
    fn enum_variants_one_per_line() {
        assert_eq!(
            item("enum Status { Active, Idle(u32), Custom { name: String } }"),
            "enum Status {\n    Active,\n    Idle(u32),\n    Custom { name: String },\n}",
        );
    }

    #[test]
    fn enum_with_discriminants() {
        assert_eq!(
            item("enum Code { Ok = 0, Err = 1 }"),
            "enum Code {\n    Ok = 0,\n    Err = 1,\n}",
        );
    }

    #[test]
    fn impl_block() {
        assert_eq!(
            item("impl Point { fn origin() -> Self { Point { x: 0.0, y: 0.0 } } }"),
            "impl Point {\n    fn origin() -> Self {\n        Point { x: 0.0, y: 0.0 }\n    }\n}",
        );
    }

    #[test]
    fn trait_impl_block() {
        assert_eq!(
            item("impl<T> Default for Wrapper<T> { fn default() -> Self { todo!() } }"),
            "impl<T> Default for Wrapper<T> {\n    fn default() -> Self {\n        todo!()\n    }\n}",
        );
    }

    #[test]
    fn empty_impl() {
        assert_eq!(item("impl Marker for Point {}"), "impl Marker for Point {}");
    }

    #[test]
    fn impl_with_const_and_type() {
        assert_eq!(
            item("impl Config { const LIMIT: usize = 16; }"),
            "impl Config {\n    const LIMIT: usize = 16;\n}",
        );
        assert_eq!(
            item("impl Iterator for Counter { type Item = u32; }"),
            "impl Iterator for Counter {\n    type Item = u32;\n}",
        );
    }

    #[test]
    fn trait_definition() {
        assert_eq!(
            item(
                "trait Greet: Display { fn greet(&self) -> String; fn loud(&self) -> String { self.greet().to_uppercase() } }"
            ),
            "trait Greet: Display {\n    fn greet(&self) -> String;\n    fn loud(&self) -> String {\n        self.greet().to_uppercase()\n    }\n}",
        );
    }

    #[test]
    fn trait_with_associated_type() {
        assert_eq!(
            item("trait Container { type Item: Clone; }"),
            "trait Container {\n    type Item: Clone;\n}",
        );
    }

    #[test]
    fn use_declarations() {
        assert_eq!(item("use std::fmt;"), "use std::fmt;");
        assert_eq!(
            item("use std::collections::{HashMap, HashSet};"),
            "use std::collections::{HashMap, HashSet};",
        );
        assert_eq!(item("use prelude::*;"), "use prelude::*;");
        assert_eq!(
            item("use std::io::Result as IoResult;"),
            "use std::io::Result as IoResult;"
        );
        assert_eq!(item("pub use crate::Error;"), "pub use crate::Error;");
    }

    #[test]
    fn long_use_group_breaks() {
        assert_eq!(
            item(
                "use crate::components::{FirstLongComponentName, SecondLongComponentName, ThirdLongComponentName};"
            ),
            "use crate::components::{\n    FirstLongComponentName,\n    SecondLongComponentName,\n    ThirdLongComponentName,\n};",
        );
    }

    #[test]
    fn const_and_static() {
        assert_eq!(
            item("const LIMIT: usize = 640;"),
            "const LIMIT: usize = 640;"
        );
        assert_eq!(
            item("static mut COUNTER: u64 = 0;"),
            "static mut COUNTER: u64 = 0;",
        );
    }

    #[test]
    fn type_alias() {
        assert_eq!(
            item("type Result<T> = std::result::Result<T, Error>;"),
            "type Result<T> = std::result::Result<T, Error>;",
        );
    }

    #[test]
    fn module() {
        assert_eq!(item("mod helpers;"), "mod helpers;");
        assert_eq!(
            item("mod helpers { fn assist() {} }"),
            "mod helpers {\n    fn assist() {}\n}",
        );
    }

    #[test]
    fn macro_rules_body_is_kept_verbatim() {
        assert_eq!(
            item("macro_rules! square {\n    ($x:expr) => { $x * $x };\n}"),
            "macro_rules! square {\n    ($x:expr) => { $x * $x };\n}",
        );
    }

    #[test]
    fn exotic_item_is_kept_verbatim() {
        assert_eq!(
            item("union Value { int: i64, float: f64 }"),
            "union Value { int: i64, float: f64 }",
        );
    }

    #[test]
    fn doc_comments_between_items() {
        assert_eq!(
            format::<syn::Block>(
                "{\n    /// First helper.\n    fn one() {}\n\n    /// Second helper.\n    fn two() {}\n}"
            ),
            "{\n    /// First helper.\n    fn one() {}\n\n    /// Second helper.\n    fn two() {}\n}",
        );
    }
}
