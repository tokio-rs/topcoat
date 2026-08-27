use proc_macro2::{TokenStream, TokenTree};
use quote::ToTokens;
use syn::{Expr, Ident, Pat, Token};

use super::Scope;

/// The bindings the control-flow patterns enclosing a scope introduce into
/// it.
///
/// A component's children under such patterns must own these values,
/// because they die with the branch or iteration that produced them while
/// the children live on in the component's props.
/// [`Scope::emit_child`](super::Scope::emit_child) moves them into the
/// children through `Capture`.
#[derive(Clone)]
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

    /// Returns the bindings `scope` mentions anywhere in its expressions,
    /// which are the ones it may borrow.
    pub(crate) fn mentioned_in(&self, scope: &Scope) -> Self {
        Self(
            self.0
                .iter()
                .filter(|binding| scope.mentions(&binding.ident))
                .cloned()
                .collect(),
        )
    }

    /// Returns these bindings extended by `other`, for a scope nested under
    /// one more pattern.
    ///
    /// A name bound again by the inner pattern shadows the outer binding,
    /// so it is kept once.
    pub(crate) fn with(&self, other: Self) -> Self {
        let mut bindings = self.clone();
        for binding in other.0 {
            bindings.push(binding);
        }
        bindings
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

/// Whether `ident` appears anywhere in `tokens`, at any nesting depth.
///
/// A string literal counts when it holds `{ident}` or `{ident:`, the
/// inline argument of a formatting macro, which borrows the name like an
/// expression would.
pub(crate) fn mentions(tokens: &TokenStream, ident: &Ident) -> bool {
    tokens.clone().into_iter().any(|tree| match tree {
        TokenTree::Ident(other) => other == *ident,
        TokenTree::Group(group) => mentions(&group.stream(), ident),
        TokenTree::Literal(literal) => {
            let literal = literal.to_string();
            literal.contains(&format!("{{{ident}}}")) || literal.contains(&format!("{{{ident}:"))
        }
        TokenTree::Punct(_) => false,
    })
}

/// Whether `tokens` contain an `.await`, at any nesting depth.
///
/// The body of an `expr!` invocation is left out: a runtime expression is
/// compiled to JavaScript, so an `await` in it never waits on the server.
pub(crate) fn awaits(tokens: &TokenStream) -> bool {
    let mut previous: [Option<TokenTree>; 2] = [None, None];
    for tree in tokens.clone() {
        let found = match &tree {
            TokenTree::Ident(ident) => ident == "await",
            TokenTree::Group(group) => {
                let runtime = matches!(
                    &previous,
                    [Some(TokenTree::Ident(name)), Some(TokenTree::Punct(bang))]
                        if name == "expr" && bang.as_char() == '!'
                );
                !runtime && awaits(&group.stream())
            }
            TokenTree::Punct(_) | TokenTree::Literal(_) => false,
        };
        if found {
            return true;
        }
        previous = [previous[1].take(), Some(tree)];
    }
    false
}

/// A single bound identifier with the mutability of its binding.
///
/// Its tokens are the rebinding pattern inside the children's body: the
/// identifier keeps its span, so lints on the binding still point at the
/// source pattern.
#[derive(Clone)]
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
    fn mentions_finds_an_ident_at_any_depth() {
        let ident: Ident = syn::parse_quote!(x);
        assert!(mentions(&quote! { f(a, (b, [x.y])) }, &ident));
        assert!(!mentions(&quote! { f(a, (b, [xy])) }, &ident));
        assert!(!mentions(&quote! { "x" }, &ident));
    }

    #[test]
    fn awaits_finds_an_await_at_any_depth() {
        assert!(awaits(&quote! { f(g().await?) }));
        assert!(!awaits(&quote! { f(g()) }));
    }

    #[test]
    fn awaits_leaves_runtime_expressions_alone() {
        assert!(!awaits(
            &quote! { ::topcoat_runtime_macro::expr! { async |e| { f().await } } }
        ));
        assert!(awaits(&quote! { g(expr! { a }, h().await) }));
    }

    #[test]
    fn mentions_finds_an_inline_format_argument() {
        let ident: Ident = syn::parse_quote!(x);
        assert!(mentions(&quote! { format!("{x}") }, &ident));
        assert!(mentions(&quote! { format!("{x:?}") }, &ident));
        assert!(!mentions(&quote! { format!("{xy}") }, &ident));
    }

    #[test]
    fn extending_keeps_a_shadowed_name_once() {
        let outer = Bindings::of_pattern(&parse_pattern("(a, b)"));
        let inner = Bindings::of_pattern(&parse_pattern("Some(b) | None"));
        let idents: Vec<_> = outer.with(inner).idents().map(Ident::to_string).collect();
        assert_eq!(idents, ["a", "b"]);
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
