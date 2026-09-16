use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::ExprIndex;

use super::js::Js;
use crate::expr::{Expr, name_resolver::NameResolver};

impl Expr {
    pub(super) fn expr_index(
        index: &ExprIndex,
        rust: &mut TokenStream,
        js: &mut Js,
        names: &mut NameResolver,
    ) -> syn::Result<()> {
        let mut base = TokenStream::new();
        js.push('(');
        Self::dispatch(&index.expr, &mut base, js, names)?;

        js.push_str(").index(");
        let mut subscript = TokenStream::new();
        Self::dispatch(&index.index, &mut subscript, js, names)?;
        js.push_str(").deref()");

        quote! { (*(#base).index(#subscript)) }.to_tokens(rust);
        Ok(())
    }
}
