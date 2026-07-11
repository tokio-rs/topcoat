use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Expr as SynExpr, ExprClosure};

use crate::expr::{
    Expr,
    name_resolver::{LocalBindingKind, NameResolver},
};

impl Expr {
    pub(super) fn expr_closure(
        closure: &ExprClosure,
        rust: &mut TokenStream,
        js: &mut String,
        names: &mut NameResolver,
    ) -> syn::Result<()> {
        let asyncness = &closure.asyncness;
        if asyncness.is_some() {
            js.push_str("async ");
        }

        js.push('(');
        names.push_scope();
        let mut inputs = Vec::with_capacity(closure.inputs.len());
        for (i, input) in closure.inputs.iter().enumerate() {
            if i > 0 {
                js.push_str(", ");
            }
            let mut tokens = TokenStream::new();
            let (ident, name) = Self::pat(input, &mut tokens, js, names)?;
            names.bind_local(&ident, name, LocalBindingKind::Plain)?;
            inputs.push(tokens);
        }
        js.push_str(") => ");

        let mut body = TokenStream::new();
        match &*closure.body {
            // A block body maps directly onto the arrow function body without
            // the IIFE wrapper that a block expression would need.
            SynExpr::Block(block) => Self::block(&block.block, &mut body, js, names)?,
            other => Self::dispatch(other, &mut body, js, names)?,
        }
        names.pop_scope();

        let capture = &closure.capture;
        let output = &closure.output;
        quote! { #asyncness #capture |#(#inputs),*| #output #body }.to_tokens(rust);
        Ok(())
    }
}
