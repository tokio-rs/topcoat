use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::ExprWhile;
use topcoat_core_grammar::common::paths::topcoat_runtime;

use super::js::Js;
use crate::expr::{Expr, NameResolver, name_resolver::JumpScope};

impl Expr {
    pub(super) fn expr_while(
        expr: &ExprWhile,
        rust: &mut TokenStream,
        js: &mut Js,
        names: &mut NameResolver,
    ) -> syn::Result<()> {
        let mut cond = TokenStream::new();
        js.push_str("while (");
        names.within(JumpScope::Function, |names| {
            Self::dispatch(&expr.cond, &mut cond, js, names)
        })?;
        js.push_str(".dehydrate()) ");
        quote! { while #topcoat_runtime::Surrogate::into_real(#cond) }.to_tokens(rust);

        names.within(JumpScope::Loop, |names| {
            Self::block(&expr.body, rust, js, names)
        })?;
        Ok(())
    }
}
