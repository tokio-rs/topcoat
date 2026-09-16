use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::ExprLoop;

use super::js::Js;
use crate::expr::{Expr, NameResolver};

impl Expr {
    pub(super) fn expr_loop(
        expr: &ExprLoop,
        rust: &mut TokenStream,
        js: &mut Js,
        names: &mut NameResolver,
    ) -> syn::Result<()> {
        js.push_str("while (true) ");
        quote! { loop }.to_tokens(rust);
        Self::block(&expr.body, rust, js, names)?;
        Ok(())
    }
}
