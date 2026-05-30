use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::ExprIndex;

use crate::ast::expr::{Expr, name_resolver::NameResolver};

impl Expr {
    pub(super) fn expr_index(
        index: &ExprIndex,
        rust: &mut TokenStream,
        js: &mut String,
        names: &mut NameResolver,
    ) -> syn::Result<()> {
        let mut base = TokenStream::new();
        Self::dispatch(&index.expr, &mut base, js, names)?;

        js.push('[');
        let mut subscript = TokenStream::new();
        Self::dispatch(&index.index, &mut subscript, js, names)?;
        js.push(']');

        quote! { (#base)[#subscript] }.to_tokens(rust);
        Ok(())
    }
}
