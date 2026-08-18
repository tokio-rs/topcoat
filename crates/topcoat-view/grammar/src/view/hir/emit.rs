use proc_macro2::{Span, TokenStream};
use quote::{TokenStreamExt, format_ident, quote, quote_spanned};
use syn::{Expr, Ident, Pat, Token};
use topcoat_core_grammar::paths::topcoat_view;

/// HIR nodes that emit themselves into the two phases of an [`Emitter`].
pub(crate) trait Emit {
    fn emit(&self, emitter: &mut Emitter);
}

/// Collects the two phases a view expression expands to.
///
/// The hoist phase evaluates every expression of the view in source order,
/// including building the views of nested control flow, and binds the results
/// to fresh identifiers. The burst phase then pushes the view's instruction
/// block in one synchronous burst that only reads those bindings. Keeping
/// every `await`, and every user expression that may build a view of its
/// own, out of the burst phase is what lets the block land contiguously in
/// the scope's shared view buffer.
///
/// Component renders are futures. With inline awaits, the hoist phase awaits
/// each one where it is bound. Without them, the hoist phase only binds the
/// futures and registers them to be joined, so sibling components render
/// concurrently; the join runs after the hoist phase, right before the
/// burst.
pub(crate) struct Emitter {
    hoist: TokenStream,
    burst: TokenStream,
    counter: u32,
    inline_await: bool,
    join: Vec<Ident>,
}

impl Emitter {
    pub(super) fn new(inline_await: bool) -> Self {
        Self {
            hoist: TokenStream::new(),
            burst: TokenStream::new(),
            counter: 0,
            inline_await,
            join: Vec::new(),
        }
    }

    /// Whether hoisted futures are awaited where they are bound instead of
    /// being joined after the hoist phase.
    pub(super) fn inline_await(&self) -> bool {
        self.inline_await
    }

    /// Returns a fresh identifier for a hoisted binding.
    pub(super) fn fresh_ident(&mut self) -> Ident {
        let ident = format_ident!("__expr{}", self.counter);
        self.counter += 1;
        ident
    }

    /// Appends statements to the hoist phase.
    pub(super) fn hoist(&mut self, tokens: TokenStream) {
        self.hoist.append_all(tokens);
    }

    /// Hoists a binding for `future`, a future of `Result`.
    ///
    /// With inline awaits the binding awaits the future in place. Otherwise
    /// the binding holds the future itself and is rebound to its output by
    /// the join that follows the hoist phase.
    pub(super) fn hoist_future(&mut self, span: Span, ident: &Ident, future: &TokenStream) {
        if self.inline_await {
            self.hoist(quote_spanned! {span=> let #ident = #future.await?; });
        } else {
            self.hoist(quote_spanned! {span=> let #ident = #future; });
            self.join.push(ident.clone());
        }
    }

    /// Appends pushes on the `__b` builder to the burst phase.
    pub(super) fn burst(&mut self, tokens: TokenStream) {
        self.burst.append_all(tokens);
    }

    /// Returns the statement that awaits the registered futures and rebinds
    /// their identifiers to the outputs, joining when there is more than one.
    fn join_bindings(&self) -> TokenStream {
        match self.join.as_slice() {
            [] => TokenStream::new(),
            [ident] => quote! { let #ident = #ident.await?; },
            idents => quote! {
                let (#(#idents),*) = #topcoat_view::internal::try_join!(#(#idents),*)?;
            },
        }
    }

    /// Returns the view expression: the hoist phase, the join of any
    /// registered futures, and the burst that builds the instruction block
    /// and yields the view handle.
    pub(super) fn finish(self) -> TokenStream {
        let join = self.join_bindings();
        let Self { hoist, burst, .. } = self;
        quote! {{
            #hoist
            #join
            #topcoat_view::internal::block(__cx, |__b| {
                #burst
            })
        }}
    }

    /// Hoists `Option` cells for `bindings` so a joined `if`/`match` arm can
    /// move them into its future.
    ///
    /// Pattern bindings die when the arm returns the future that captures
    /// them. The cells live in the enclosing hoist, the arm stashes each
    /// binding into one, and the future's prelude takes them back out, so the
    /// coroutine owns them for as long as it runs.
    ///
    /// Returns the arm-body stash assignments and the prelude that rebinds
    /// them inside the future.
    pub(super) fn stash_bindings(&mut self, bindings: &[Binding]) -> (TokenStream, TokenStream) {
        let mut stash = TokenStream::new();
        let mut prelude = TokenStream::new();
        for binding in bindings {
            let temp = self.fresh_ident();
            self.hoist(quote! {
                let mut #temp = ::core::option::Option::None;
            });
            let ident = &binding.ident;
            let mutability = &binding.mutability;
            stash.append_all(quote! {
                #temp = ::core::option::Option::Some(#ident);
            });
            prelude.append_all(quote! {
                let #mutability #ident = #temp.take().unwrap();
            });
        }
        (stash, prelude)
    }
}

/// A named binding introduced by an `if let` condition or `match` arm pattern.
pub(crate) struct Binding {
    ident: Ident,
    mutability: Option<Token![mut]>,
}

/// Bindings introduced by `if let` / let-chain conditions.
pub(crate) fn condition_bindings(expr: &Expr) -> Vec<Binding> {
    let mut bindings = Vec::new();
    collect_condition_bindings(expr, &mut bindings);
    bindings
}

/// Bindings introduced by a `match` arm pattern.
pub(crate) fn pattern_bindings(pat: &Pat) -> Vec<Binding> {
    let mut bindings = Vec::new();
    collect_pat_bindings(pat, &mut bindings);
    bindings
}

fn collect_condition_bindings(expr: &Expr, bindings: &mut Vec<Binding>) {
    match expr {
        Expr::Let(expr) => collect_pat_bindings(&expr.pat, bindings),
        Expr::Binary(expr) => {
            collect_condition_bindings(&expr.left, bindings);
            collect_condition_bindings(&expr.right, bindings);
        }
        Expr::Paren(expr) => collect_condition_bindings(&expr.expr, bindings),
        Expr::Group(expr) => collect_condition_bindings(&expr.expr, bindings),
        _ => {}
    }
}

fn collect_pat_bindings(pat: &Pat, bindings: &mut Vec<Binding>) {
    match pat {
        Pat::Ident(pat) => {
            if is_binding_ident(pat) && bindings.iter().all(|binding| binding.ident != pat.ident) {
                bindings.push(Binding {
                    ident: pat.ident.clone(),
                    mutability: pat.mutability,
                });
            }
            if let Some((_, subpat)) = &pat.subpat {
                collect_pat_bindings(subpat, bindings);
            }
        }
        Pat::Or(pat) => {
            for case in &pat.cases {
                collect_pat_bindings(case, bindings);
            }
        }
        Pat::Tuple(pat) => {
            for elem in &pat.elems {
                collect_pat_bindings(elem, bindings);
            }
        }
        Pat::TupleStruct(pat) => {
            for elem in &pat.elems {
                collect_pat_bindings(elem, bindings);
            }
        }
        Pat::Struct(pat) => {
            for field in &pat.fields {
                collect_pat_bindings(&field.pat, bindings);
            }
        }
        Pat::Slice(pat) => {
            for elem in &pat.elems {
                collect_pat_bindings(elem, bindings);
            }
        }
        Pat::Reference(pat) => collect_pat_bindings(&pat.pat, bindings),
        Pat::Paren(pat) => collect_pat_bindings(&pat.pat, bindings),
        Pat::Type(pat) => collect_pat_bindings(&pat.pat, bindings),
        _ => {}
    }
}

/// Whether this ident introduces a variable rather than naming a unit
/// variant or constant.
///
/// A single uppercase ident in pattern position is almost always a path
/// like `None` or `Status::First`'s `First`, not a binding. `ref`, `mut`,
/// and `@` make the ident a binding regardless of case.
fn is_binding_ident(pat: &syn::PatIdent) -> bool {
    if pat.ident == "_" {
        return false;
    }
    if pat.by_ref.is_some() || pat.mutability.is_some() || pat.subpat.is_some() {
        return true;
    }
    !pat.ident.to_string().starts_with(char::is_uppercase)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(pat: &str) -> Vec<String> {
        pattern_bindings(&syn::parse::Parser::parse_str(Pat::parse_single, pat).unwrap())
            .into_iter()
            .map(|binding| binding.ident.to_string())
            .collect()
    }

    #[test]
    fn pattern_bindings_collect_lowercase_idents() {
        assert_eq!(names("Some(status)"), vec!["status"]);
        assert_eq!(names("(a, mut b)"), vec!["a", "b"]);
        assert_eq!(names("s @ Some(inner)"), vec!["s", "inner"]);
    }

    #[test]
    fn pattern_bindings_skip_unit_variants() {
        assert!(names("None").is_empty());
        assert!(names("Status::First").is_empty());
    }

    #[test]
    fn condition_bindings_collect_if_let_and_let_chains() {
        let expr: Expr = syn::parse_quote!(let Some(status) = opt);
        let names: Vec<_> = condition_bindings(&expr)
            .into_iter()
            .map(|binding| binding.ident.to_string())
            .collect();
        assert_eq!(names, vec!["status"]);

        let expr: Expr = syn::parse_quote!(let Some(a) = x && let Some(b) = y);
        let names: Vec<_> = condition_bindings(&expr)
            .into_iter()
            .map(|binding| binding.ident.to_string())
            .collect();
        assert_eq!(names, vec!["a", "b"]);
    }
}
