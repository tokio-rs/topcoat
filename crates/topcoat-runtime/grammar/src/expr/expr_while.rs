use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::ExprWhile;
use topcoat_core_grammar::paths::topcoat_runtime;

use crate::expr::{Expr, NameResolver};

impl Expr {
    pub(super) fn expr_while(
        expr: &ExprWhile,
        rust: &mut TokenStream,
        js: &mut String,
        names: &mut NameResolver,
    ) -> syn::Result<()> {
        let mut cond = TokenStream::new();
        js.push_str("while (");
        Self::dispatch(&expr.cond, &mut cond, js, names)?;
        js.push_str(".dehydrate()) ");
        quote! { while #topcoat_runtime::Surrogate::into_real(#cond) }.to_tokens(rust);

        Self::block(&expr.body, rust, js, names)?;
        Ok(())
    }
}
