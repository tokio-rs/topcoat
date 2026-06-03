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

    #[allow(dead_code)]
    pub(super) fn chunks(&self) -> &[Chunk] {
        &self.chunks
    }

    /// Records a single `__attrs.insert(key, value);` call.
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
                    Chunk::Insert { tokens, .. } => tokens.to_tokens(&mut output),
                    Chunk::Let { pat, expr } => quote! { let #pat = #expr; }.to_tokens(&mut output),
                    Chunk::Statement { tokens } => tokens.to_tokens(&mut output),
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
            Chunk::Let { .. } | Chunk::Statement { .. } => 0,
            Chunk::For { .. } => 0,
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
