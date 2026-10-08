use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::ExprTuple;

use super::js::Js;
use crate::expr::{Expr, name_resolver::NameResolver};

impl Expr {
    pub(super) fn expr_tuple(
        tuple: &ExprTuple,
        rust: &mut TokenStream,
        js: &mut Js,
        names: &mut NameResolver,
    ) -> syn::Result<()> {
        if let Some(attr) = tuple.attrs.first() {
            return Err(syn::Error::new_spanned(
                attr,
                "attributes are not supported",
            ));
        }

        if tuple.elems.is_empty() {
            quote! { () }.to_tokens(rust);
            js.push_str("undefined");
            return Ok(());
        }

        js.push_str("cx.tuple([");
        let mut elements = Vec::with_capacity(tuple.elems.len());
        for (index, element) in tuple.elems.iter().enumerate() {
            if index > 0 {
                js.push_str(", ");
            }
            let mut value = TokenStream::new();
            Self::dispatch(element, &mut value, js, names)?;
            elements.push(value);
        }
        js.push_str("])");
        quote! { (#(#elements,)*) }.to_tokens(rust);
        Ok(())
    }
}
