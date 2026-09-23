#![cfg_attr(docsrs, feature(doc_cfg))]

use proc_macro::TokenStream;
use quote::quote;

#[doc = include_str!("../docs/expr.md")]
#[proc_macro]
pub fn expr(tokens: TokenStream) -> TokenStream {
    let parsed = syn::parse_macro_input!(tokens as topcoat_runtime_grammar::expr::Expr);
    match parsed.expr_to_tokens() {
        Ok(ts) => ts.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

#[doc = include_str!("../docs/procedure.md")]
#[proc_macro_attribute]
pub fn procedure(attr: TokenStream, item: TokenStream) -> TokenStream {
    match topcoat_runtime_grammar::procedure::Procedure::parse(attr.into(), item.into()) {
        Ok(value) => quote! { #value }.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

#[doc = include_str!("../docs/shard.md")]
#[proc_macro_attribute]
pub fn shard(attr: TokenStream, item: TokenStream) -> TokenStream {
    match topcoat_runtime_grammar::shard::Shard::parse(attr.into(), item.into()) {
        Ok(value) => quote! { #value }.into(),
        Err(error) => error.to_compile_error().into(),
    }
}
