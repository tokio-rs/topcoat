use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Expr, Pat};

/// AST nodes that can emit themselves into an [`AttributeWriter`].
pub(crate) trait WriteAttribute {
    fn write(&self, writer: &mut AttributeWriter);
}

/// Builds the `TokenStream` that an [`Attributes`](super::Attributes) list
/// expands to.
///
/// Each `__attrs.insert(...)` call is recorded as an [`Chunk::Insert`] along
/// with how many entries it contributes; control-flow chunks (`if`/`for`/
/// `match`) recurse into nested writers. The capacity hint passed to
/// `Attributes::with_capacity` is derived from these recorded contributions.
pub(crate) struct AttributeWriter {
    chunks: Vec<Chunk>,
}

impl AttributeWriter {
    pub fn new() -> Self {
        Self { chunks: Vec::new() }
    }

    /// Records a single `__attrs.insert(key, value);` call.
    #[allow(clippy::needless_pass_by_value)]
    pub fn insert(&mut self, key: TokenStream, value: TokenStream) {
        self.chunks.push(Chunk::Insert {
            tokens: quote! { __attrs.insert(#key, #value); },
            capacity: 1,
        });
    }

    /// Records a self-contained block that performs `capacity` inserts into
    /// `__attrs`.
    pub fn insert_block(&mut self, capacity: usize, tokens: TokenStream) {
        self.chunks.push(Chunk::Insert { tokens, capacity });
    }

    pub fn let_binding(&mut self, pat: &Pat, expr: &Expr) {
        self.chunks.push(Chunk::Let {
            pat: pat.clone(),
            expr: Box::new(expr.clone()),
        });
    }

    pub fn statement(&mut self, tokens: TokenStream) {
        self.chunks.push(Chunk::Statement { tokens });
    }

    pub fn for_loop(&mut self, pat: &Pat, expr: &Expr, f: impl FnOnce(&mut AttributeWriter)) {
        let mut body = AttributeWriter::new();
        f(&mut body);
        self.chunks.push(Chunk::For {
            pat: pat.clone(),
            expr: Box::new(expr.clone()),
            body: Box::new(body),
        });
    }

    pub fn if_else(
        &mut self,
        cond: &Expr,
        f: impl FnOnce(&mut AttributeWriter, &mut AttributeWriter),
    ) {
        let mut then_branch = AttributeWriter::new();
        let mut else_branch = AttributeWriter::new();
        f(&mut then_branch, &mut else_branch);
        self.chunks.push(Chunk::If {
            cond: cond.clone(),
            then_branch: Box::new(then_branch),
            else_branch: Box::new(else_branch),
        });
    }

    pub fn match_expr(&mut self, expr: &Expr, f: impl FnOnce(&mut MatchArmsBuilder)) {
        let mut builder = MatchArmsBuilder { arms: Vec::new() };
        f(&mut builder);
        self.chunks.push(Chunk::Match {
            expr: Box::new(expr.clone()),
            arms: builder.arms,
        });
    }

    pub fn into_token_stream(self) -> TokenStream {
        fn build_parts(chunks: &[Chunk]) -> TokenStream {
            let mut output = TokenStream::new();
            for chunk in chunks {
                match chunk {
                    Chunk::Insert { tokens, .. } | Chunk::Statement { tokens } => {
                        tokens.to_tokens(&mut output);
                    }
                    Chunk::Let { pat, expr } => quote! { let #pat = #expr; }.to_tokens(&mut output),
                    Chunk::For { pat, expr, body } => {
                        let body = build_parts(&body.chunks);
                        quote! {
                            for #pat in #expr {
                                #body
                            }
                        }
                        .to_tokens(&mut output);
                    }
                    Chunk::If {
                        cond,
                        then_branch,
                        else_branch,
                    } => {
                        let then_tokens = build_parts(&then_branch.chunks);
                        let else_tokens = (!else_branch.chunks.is_empty()).then(|| {
                            let body = build_parts(&else_branch.chunks);
                            quote! { else { #body } }
                        });
                        quote! {
                            if #cond {
                                #then_tokens
                            }
                            #else_tokens
                        }
                        .to_tokens(&mut output);
                    }
                    Chunk::Match { expr, arms } => {
                        let arm_tokens = arms.iter().map(|arm| {
                            let pat = &arm.pat;
                            let guard = arm.guard.as_ref().map(|g| quote! { if #g });
                            let body = build_parts(&arm.body.chunks);
                            quote! { #pat #guard => { #body } }
                        });
                        quote! {
                            match #expr {
                                #(#arm_tokens,)*
                            }
                        }
                        .to_tokens(&mut output);
                    }
                }
            }
            output
        }

        let capacity = Chunk::capacity_of(&self.chunks);
        let statements = build_parts(&self.chunks);
        quote! {{
            let mut __attrs = ::topcoat::view::Attributes::with_capacity(#capacity);
            #statements
            __attrs
        }}
    }
}

pub(super) enum Chunk {
    Insert {
        tokens: TokenStream,
        capacity: usize,
    },
    Let {
        pat: Pat,
        expr: Box<Expr>,
    },
    Statement {
        tokens: TokenStream,
    },
    For {
        pat: Pat,
        expr: Box<Expr>,
        body: Box<AttributeWriter>,
    },
    If {
        cond: Expr,
        then_branch: Box<AttributeWriter>,
        else_branch: Box<AttributeWriter>,
    },
    Match {
        expr: Box<Expr>,
        arms: Vec<MatchArm>,
    },
}

impl Chunk {
    pub fn capacity_of(chunks: &[Chunk]) -> usize {
        chunks.iter().map(Chunk::capacity).sum()
    }

    fn capacity(&self) -> usize {
        match self {
            Chunk::Insert { capacity, .. } => *capacity,
            Chunk::Let { .. } | Chunk::Statement { .. } | Chunk::For { .. } => 0,
            Chunk::If {
                then_branch,
                else_branch,
                ..
            } => {
                Chunk::capacity_of(&then_branch.chunks).min(Chunk::capacity_of(&else_branch.chunks))
            }
            Chunk::Match { arms, .. } => arms
                .iter()
                .map(|arm| Chunk::capacity_of(&arm.body.chunks))
                .min()
                .unwrap_or_default(),
        }
    }
}

pub(super) struct MatchArm {
    pat: Pat,
    guard: Option<Expr>,
    body: Box<AttributeWriter>,
}

pub(crate) struct MatchArmsBuilder {
    arms: Vec<MatchArm>,
}

impl MatchArmsBuilder {
    pub fn arm(&mut self, pat: &Pat, guard: Option<&Expr>, f: impl FnOnce(&mut AttributeWriter)) {
        let mut body = AttributeWriter::new();
        f(&mut body);
        self.arms.push(MatchArm {
            pat: pat.clone(),
            guard: guard.cloned(),
            body: Box::new(body),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rendered(writer: AttributeWriter) -> String {
        writer.into_token_stream().to_string()
    }

    #[test]
    fn empty_writer_emits_zero_capacity_block() {
        let writer = AttributeWriter::new();
        let out = rendered(writer);
        assert!(out.contains(":: topcoat :: view :: Attributes :: with_capacity (0usize)"));
        assert!(!out.contains("insert"));
    }

    #[test]
    fn insert_records_one_capacity_per_entry() {
        let mut writer = AttributeWriter::new();
        writer.insert(quote! { "class" }, quote! { "btn" });
        writer.insert(quote! { "id" }, quote! { "x" });
        let out = rendered(writer);
        assert!(out.contains("with_capacity (2usize)"));
        assert!(out.contains("__attrs . insert (\"class\" , \"btn\")"));
        assert!(out.contains("__attrs . insert (\"id\" , \"x\")"));
    }

    #[test]
    fn if_capacity_is_minimum_of_branches() {
        let mut writer = AttributeWriter::new();
        writer.if_else(&syn::parse_quote!(cond), |then_branch, else_branch| {
            then_branch.insert(quote! { "a" }, quote! { "1" });
            then_branch.insert(quote! { "b" }, quote! { "2" });
            else_branch.insert(quote! { "c" }, quote! { "3" });
        });
        assert!(rendered(writer).contains("with_capacity (1usize)"));
    }

    #[test]
    fn if_without_else_contributes_no_minimum_capacity() {
        let mut writer = AttributeWriter::new();
        writer.if_else(&syn::parse_quote!(cond), |then_branch, _| {
            then_branch.insert(quote! { "a" }, quote! { "1" });
        });
        let out = rendered(writer);
        assert!(out.contains("with_capacity (0usize)"));
        assert!(!out.contains("else"));
    }

    #[test]
    fn for_loop_contributes_no_static_capacity() {
        let mut writer = AttributeWriter::new();
        writer.for_loop(
            &syn::parse_quote!((k, v)),
            &syn::parse_quote!(items),
            |body| body.insert(quote! { k }, quote! { v }),
        );
        let out = rendered(writer);
        assert!(out.contains("with_capacity (0usize)"));
        assert!(out.contains("for (k , v) in items"));
    }

    #[test]
    fn match_capacity_is_minimum_over_arms() {
        let mut writer = AttributeWriter::new();
        writer.match_expr(&syn::parse_quote!(v), |arms| {
            arms.arm(&syn::parse_quote!(A), None, |body| {
                body.insert(quote! { "x" }, quote! { "1" });
            });
            arms.arm(
                &syn::parse_quote!(B),
                Some(&syn::parse_quote!(flag)),
                |body| {
                    body.insert(quote! { "x" }, quote! { "2" });
                    body.insert(quote! { "y" }, quote! { "3" });
                },
            );
        });
        let out = rendered(writer);
        assert!(out.contains("with_capacity (1usize)"));
        assert!(out.contains("match v"));
        assert!(out.contains("if flag"));
    }

    #[test]
    fn let_and_statement_are_emitted_verbatim() {
        let mut writer = AttributeWriter::new();
        writer.let_binding(&syn::parse_quote!(x), &syn::parse_quote!(value));
        writer.statement(quote! { break; });
        let out = rendered(writer);
        assert!(out.contains("let x = value"));
        assert!(out.contains("break ;"));
    }
}
