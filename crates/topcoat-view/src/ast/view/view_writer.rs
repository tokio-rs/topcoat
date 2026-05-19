use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Expr, Pat};

/// AST nodes that can emit themselves into a [`ViewWriter`].
pub(crate) trait WriteView {
    fn write(&self, writer: &mut ViewWriter);
}

/// Builds the `TokenStream` that a `view!` invocation expands to.
///
/// Adjacent literal markup is concatenated into `static_segment` and flushed as
/// a single write whenever a dynamic chunk (expression, control flow) appears.
/// `capacity` accumulates the lower bound of bytes the rendered view will need
/// so the runtime can pre-allocate the output buffer.
pub(crate) struct ViewWriter {
    pub(self) chunks: Vec<Chunk>,
    static_segment: String,
    nested: bool,
}

impl ViewWriter {
    pub fn new() -> Self {
        Self {
            chunks: Vec::new(),
            static_segment: String::new(),
            nested: false,
        }
    }

    pub fn new_nested() -> Self {
        Self {
            chunks: Vec::new(),
            static_segment: String::new(),
            nested: true,
        }
    }

    pub fn flush(&mut self) {
        if !self.static_segment.is_empty() {
            let static_segment = &self.static_segment;
            self.chunks.push(Chunk::Expr(
                quote! { ::topcoat::view::Unescaped::new_unchecked(#static_segment) },
            ));
            self.static_segment.clear();
        }
    }

    pub fn write_str_unescaped(&mut self, s: &str) {
        self.static_segment.push_str(s);
    }

    pub fn write_str(&mut self, s: &str) {
        crate::runtime::Formatter::new(&mut self.static_segment).write_str(s);
    }

    pub fn write_expr(&mut self, expr: TokenStream) {
        self.flush();
        self.chunks.push(Chunk::Expr(expr))
    }

    pub fn let_binding(&mut self, pat: &Pat, expr: &Expr) {
        self.flush();
        self.chunks.push(Chunk::Let {
            pat: pat.clone(),
            expr: Box::new(expr.clone()),
        });
    }

    pub fn for_loop(&mut self, pat: &Pat, expr: &Expr, f: impl FnOnce(&mut ViewWriter)) {
        self.flush();
        let mut body = ViewWriter::new();
        f(&mut body);
        body.flush();
        self.chunks.push(Chunk::For {
            pat: pat.clone(),
            expr: Box::new(expr.clone()),
            body: Box::new(body),
        });
    }

    pub fn if_else(&mut self, expr: &Expr, f: impl FnOnce(&mut ViewWriter, &mut ViewWriter)) {
        self.flush();
        let mut then_branch = ViewWriter::new();
        let mut else_branch = ViewWriter::new();
        f(&mut then_branch, &mut else_branch);
        then_branch.flush();
        else_branch.flush();
        self.chunks.push(Chunk::If {
            expr: expr.clone(),
            then_branch: Box::new(then_branch),
            else_branch: Box::new(else_branch),
        });
    }

    pub fn match_expr(&mut self, expr: &Expr, f: impl FnOnce(&mut MatchArmsBuilder)) {
        self.flush();
        let mut builder = MatchArmsBuilder { arms: Vec::new() };
        f(&mut builder);
        self.chunks.push(Chunk::Match {
            expr: Box::new(expr.clone()),
            arms: builder.arms,
        });
    }

    pub fn statement(&mut self, statement: TokenStream) {
        self.flush();
        self.chunks.push(Chunk::Statement(statement));
    }

    pub fn into_token_stream(mut self) -> TokenStream {
        self.flush();

        let format_expr = {
            fn needs_vec(chunks: &[Chunk]) -> bool {
                chunks.iter().any(|chunk| match chunk {
                    Chunk::Expr(_) => false,
                    Chunk::For { body, .. } => needs_vec(&body.chunks),
                    Chunk::Let { .. }
                    | Chunk::If { .. }
                    | Chunk::Match { .. }
                    | Chunk::Statement(..) => true,
                })
            }

            if self.chunks.is_empty() {
                // Optimized path: The view has no content.
                quote! { ::topcoat::view::View::empty() }
            } else if self.chunks.len() == 1 && matches!(self.chunks[0], Chunk::Expr(_)) {
                // Optimized path: The view can be constructed from a single expression.
                let Chunk::Expr(entry) = &self.chunks[0] else {
                    unreachable!()
                };
                quote! { ::topcoat::view::View::new(#entry) }
            } else if !needs_vec(&self.chunks) {
                // No `let`/`if`/`match`: build a chained iterator of parts.
                fn build_chain(chunks: &[Chunk]) -> TokenStream {
                    fn chunk_to_iter(chunk: &Chunk) -> TokenStream {
                        match chunk {
                            Chunk::Expr(expr) => quote! { IntoViewParts::into_view_parts(#expr) },
                            Chunk::For { pat, expr, body } => {
                                let body_iter = build_chain(&body.chunks);
                                quote! {
                                    ::core::iter::IntoIterator::into_iter(#expr)
                                        .flat_map(|#pat| #body_iter)
                                }
                            }
                            _ => unreachable!("`let`/`if`/`match` require the vec branch"),
                        }
                    }

                    if chunks.is_empty() {
                        return quote! {
                            ::core::iter::empty::<::topcoat::view::ViewPart>()
                        };
                    }
                    let first = chunk_to_iter(&chunks[0]);
                    let rest = chunks[1..].iter().map(chunk_to_iter);
                    quote! { #first #(.chain(#rest))* }
                }

                let chain = build_chain(&self.chunks);
                quote! {{
                    use ::topcoat::view::IntoViewParts;
                    ::core::iter::Iterator::collect::<::topcoat::view::View>(#chain)
                }}
            } else {
                // `let`/`if`/`match` need imperative control flow; build a `Vec`.
                fn build_vec(chunks: &[Chunk]) -> TokenStream {
                    let mut output = TokenStream::new();
                    for chunk in chunks {
                        match chunk {
                            Chunk::Expr(expr) => {
                                quote! { __v.extend(IntoViewParts::into_view_parts(#expr)); }
                            }
                            Chunk::Let { pat, expr } => {
                                quote! { let #pat = #expr; }
                            }
                            Chunk::If {
                                expr,
                                then_branch: then,
                                else_branch: r#else,
                            } => {
                                let then_branch = build_vec(&then.chunks);
                                let else_branch = build_vec(&r#else.chunks);
                                let else_branch = (!r#else.chunks.is_empty())
                                    .then(|| quote! { else { #else_branch } });
                                quote! {
                                    if #expr {
                                        #then_branch
                                    }
                                    #else_branch
                                }
                            }
                            Chunk::For { pat, expr, body } => {
                                let body = build_vec(&body.chunks);
                                quote! {
                                    for #pat in #expr {
                                        #body
                                    }
                                }
                            }
                            Chunk::Match { expr, arms } => {
                                let arm_tokens = arms.iter().map(|arm| {
                                    let pat = &arm.pat;
                                    let guard = arm.guard.as_ref().map(|g| quote! { if #g });
                                    let body = build_vec(&arm.body.chunks);
                                    quote! {
                                        #pat #guard => { #body }
                                    }
                                });
                                quote! {
                                    match #expr {
                                        #(#arm_tokens,)*
                                    }
                                }
                            }
                            Chunk::Statement(tokens) => tokens.clone(),
                        }
                        .to_tokens(&mut output);
                    }
                    output
                }

                let capacity = self
                    .chunks
                    .iter()
                    .filter(|chunk| matches!(chunk, Chunk::Expr(..)))
                    .count();
                let statements = build_vec(&self.chunks);

                quote! {{
                    use ::topcoat::view::IntoViewParts;
                    let mut __v = ::std::vec::Vec::with_capacity(#capacity);
                    #statements
                    ::topcoat::view::View::new(__v.into_boxed_slice())
                }}
            }
        };

        if self.nested {
            format_expr
        } else {
            quote! { async { Ok(#format_expr) }.await }
        }
    }
}

enum Chunk {
    Expr(TokenStream),
    Let {
        pat: Pat,
        expr: Box<Expr>,
    },
    For {
        pat: Pat,
        expr: Box<Expr>,
        body: Box<ViewWriter>,
    },
    If {
        expr: Expr,
        then_branch: Box<ViewWriter>,
        else_branch: Box<ViewWriter>,
    },
    Match {
        expr: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    Statement(TokenStream),
}

struct MatchArm {
    pat: Pat,
    guard: Option<Expr>,
    body: Box<ViewWriter>,
}

pub(crate) struct MatchArmsBuilder {
    arms: Vec<MatchArm>,
}

impl MatchArmsBuilder {
    pub fn arm(&mut self, pat: &Pat, guard: Option<&Expr>, f: impl FnOnce(&mut ViewWriter)) {
        let mut body = ViewWriter::new();
        f(&mut body);
        body.flush();
        self.arms.push(MatchArm {
            pat: pat.clone(),
            guard: guard.cloned(),
            body: Box::new(body),
        });
    }
}
