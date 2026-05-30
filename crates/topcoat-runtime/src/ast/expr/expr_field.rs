use std::fmt::Write;

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{ExprField, Member};

use crate::ast::expr::{Expr, name_resolver::NameResolver};

impl Expr {
    pub(super) fn expr_field(
        field: &ExprField,
        rust: &mut TokenStream,
        js: &mut String,
        names: &mut NameResolver,
    ) -> syn::Result<()> {
        let mut base = TokenStream::new();
        Self::dispatch(&field.base, &mut base, js, names)?;

        match &field.member {
            Member::Named(name) => write!(js, ".{name}").unwrap(),
            Member::Unnamed(index) => write!(js, "[{}]", index.index).unwrap(),
        }

        let member = &field.member;
        quote! { (#base).#member }.to_tokens(rust);
        Ok(())
    }
}
