use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::Expr;

use crate::view::hir::{
    Scope,
    bindings::awaits,
    emit::{Emit, Emitter},
};

/// An `if`/`else` whose branches are lowered into nested scopes.
pub(crate) struct IfElse {
    pub expr: Expr,
    pub then_branch: Scope,
    pub else_branch: Scope,
}

impl IfElse {
    /// Returns the condition with every awaiting operand evaluated with
    /// the block suspended.
    ///
    /// A `let` in the condition is not an expression, so a `let` chain is
    /// walked down to its scrutinees and plain operands, each wrapped on
    /// its own.
    fn condition(expr: &Expr) -> TokenStream {
        match expr {
            Expr::Let(let_) => {
                let pat = &let_.pat;
                let scrutinee = Self::condition(&let_.expr);
                quote! { let #pat = #scrutinee }
            }
            Expr::Binary(binary) if matches!(binary.op, syn::BinOp::And(_)) => {
                let left = Self::condition(&binary.left);
                let right = Self::condition(&binary.right);
                quote! { #left && #right }
            }
            expr => {
                let tokens = expr.to_token_stream();
                if awaits(&tokens) {
                    Emitter::awaited(&tokens)
                } else {
                    tokens
                }
            }
        }
    }
}

impl Emit for IfElse {
    fn emit(&self, emitter: &mut Emitter<'_>) {
        let Self {
            expr,
            then_branch,
            else_branch,
        } = self;
        let expr = Self::condition(expr);
        // The taken branch renders right inside the branch, where the
        // condition's bindings are alive. An empty else branch is left
        // out.
        emitter.control_flow(|emitter| {
            let then_branch = emitter.nested(then_branch);
            let else_branch = emitter.nested(else_branch);
            let else_branch = (!else_branch.is_empty()).then(|| quote! { else { #else_branch } });
            quote! {
                if #expr {
                    #then_branch
                }
                #else_branch
            }
        });
    }
}
