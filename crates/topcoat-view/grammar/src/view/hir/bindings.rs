use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{Expr, Ident, Pat, Token};

/// The bindings a control-flow pattern introduces into its body's scope.
///
/// A body's view must own these values, because they die with the branch or
/// iteration that produced them while the view lives on.
/// [`Scope::emit_captured`](super::Scope::emit_captured) moves them into the
/// view through `Capture`.
pub(crate) struct Bindings(Vec<Binding>);

impl Bindings {
    /// Returns the empty set, for a body without an enclosing pattern.
    pub(crate) fn empty() -> Self {
        Self(Vec::new())
    }

    /// Collects the bindings of a single pattern, like a `match` arm's or a
    /// `for` loop's.
    pub(crate) fn of_pattern(pat: &Pat) -> Self {
        let mut bindings = Self::empty();
        bindings.collect_pattern(pat);
        bindings
    }

    /// Collects the bindings of an `if` condition: the patterns of its
    /// `let`s, walking `&&` chains.
    pub(crate) fn of_condition(expr: &Expr) -> Self {
        let mut bindings = Self::empty();
        bindings.collect_condition(expr);
        bindings
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the bound identifiers, for packing the values into a
    /// `Capture` where the pattern's bindings are in scope.
    pub(crate) fn idents(&self) -> impl Iterator<Item = &Ident> {
        self.0.iter().map(|binding| &binding.ident)
    }

    /// Returns the rebinding patterns, for taking the values back out of
    /// the `Capture` inside the view's body.
    pub(crate) fn rebinds(&self) -> impl Iterator<Item = &Binding> {
        self.0.iter()
    }

    fn collect_condition(&mut self, expr: &Expr) {
        match expr {
            Expr::Let(let_) => self.collect_pattern(&let_.pat),
            Expr::Binary(binary) if matches!(binary.op, syn::BinOp::And(_)) => {
                self.collect_condition(&binary.left);
                self.collect_condition(&binary.right);
            }
            Expr::Paren(paren) => self.collect_condition(&paren.expr),
            Expr::Group(group) => self.collect_condition(&group.expr),
            _ => {}
        }
    }

    fn collect_pattern(&mut self, pat: &Pat) {
        match pat {
            Pat::Ident(ident) => {
                // A bare ident can also be a unit variant or a constant,
                // which syn cannot tell apart from a binding. Those follow
                // the uppercase naming convention, so an uppercase-initial
                // ident is left alone; rebinding it could illegally shadow
                // the item it names.
                let name = ident.ident.to_string();
                let name = name.strip_prefix("r#").unwrap_or(&name);
                if !name.starts_with(char::is_uppercase) {
                    self.push(Binding {
                        mutability: ident.mutability,
                        ident: ident.ident.clone(),
                    });
                }
                if let Some((_, subpat)) = &ident.subpat {
                    self.collect_pattern(subpat);
                }
            }
            Pat::Or(or) => {
                // Every alternative binds the same names, so collecting all
                // of them relies on `push` deduplicating.
                for case in &or.cases {
                    self.collect_pattern(case);
                }
            }
            Pat::Paren(paren) => self.collect_pattern(&paren.pat),
            Pat::Reference(reference) => self.collect_pattern(&reference.pat),
            Pat::Slice(slice) => {
                for elem in &slice.elems {
                    self.collect_pattern(elem);
                }
            }
            Pat::Struct(struct_) => {
                for field in &struct_.fields {
                    self.collect_pattern(&field.pat);
                }
            }
            Pat::Tuple(tuple) => {
                for elem in &tuple.elems {
                    self.collect_pattern(elem);
                }
            }
            Pat::TupleStruct(tuple_struct) => {
                for elem in &tuple_struct.elems {
                    self.collect_pattern(elem);
                }
            }
            Pat::Type(type_) => self.collect_pattern(&type_.pat),
            _ => {}
        }
    }

    fn push(&mut self, binding: Binding) {
        if !self.0.iter().any(|other| other.ident == binding.ident) {
            self.0.push(binding);
        }
    }
}

/// A single bound identifier with the mutability of its binding.
///
/// Its tokens are the rebinding pattern inside the view's body: the
/// identifier keeps its span, so lints on the binding still point at the
/// source pattern.
pub(crate) struct Binding {
    mutability: Option<Token![mut]>,
    ident: Ident,
}

impl ToTokens for Binding {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.mutability.to_tokens(tokens);
        self.ident.to_tokens(tokens);
    }
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use super::*;

    fn pattern_idents(pat: &Pat) -> Vec<String> {
        let bindings = Bindings::of_pattern(pat);
        bindings.idents().map(Ident::to_string).collect()
    }

    fn condition_idents(expr: &Expr) -> Vec<String> {
        let bindings = Bindings::of_condition(expr);
        bindings.idents().map(Ident::to_string).collect()
    }

    fn parse_pattern(source: &str) -> Pat {
        syn::parse::Parser::parse_str(Pat::parse_multi, source).unwrap()
    }

    #[test]
    fn collects_bindings_from_nested_patterns() {
        let pat = parse_pattern("Some((a, Obj { name, .. }, [b, ..], &mut c))");
        assert_eq!(pattern_idents(&pat), ["a", "name", "b", "c"]);
    }

    #[test]
    fn keeps_the_mut_of_a_binding() {
        let pat = parse_pattern("Some(mut attrs)");
        let bindings = Bindings::of_pattern(&pat);
        let rebinds: Vec<_> = bindings.rebinds().collect();
        assert_eq!(quote! { #(#rebinds),* }.to_string(), "mut attrs");
    }

    #[test]
    fn skips_uppercase_idents() {
        let pat = parse_pattern("(status, None, Ordering)");
        assert_eq!(pattern_idents(&pat), ["status"]);
    }

    #[test]
    fn collects_an_at_binding_and_its_subpattern() {
        let pat = parse_pattern("whole @ Some(part)");
        assert_eq!(pattern_idents(&pat), ["whole", "part"]);
    }

    #[test]
    fn deduplicates_across_or_alternatives() {
        let pat = parse_pattern("Ok(value) | Err(value)");
        assert_eq!(pattern_idents(&pat), ["value"]);
    }

    #[test]
    fn collects_let_patterns_from_a_condition_chain() {
        let expr = syn::parse_quote! {
            flag && let Some(a) = first && let Some(b) = second
        };
        assert_eq!(condition_idents(&expr), ["a", "b"]);
    }

    #[test]
    fn a_plain_condition_has_no_bindings() {
        let expr = syn::parse_quote! { items.len() > 2 };
        assert!(Bindings::of_condition(&expr).is_empty());
    }
}
