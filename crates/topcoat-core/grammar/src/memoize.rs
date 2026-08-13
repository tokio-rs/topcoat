use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote, quote_spanned};
use syn::{
    FnArg, ItemFn, Pat, ReturnType,
    parse::{Parse, ParseStream},
    parse_quote,
    spanned::Spanned,
};

use crate::paths::topcoat_context;

mod kw {
    use syn::custom_keyword;

    custom_keyword!(as_ref);
}

pub struct MemoizeAttr {
    as_ref: Option<kw::as_ref>,
}

impl MemoizeAttr {
    /// Whether the return value is borrowed through `.as_ref()`, exposing the
    /// `MemoizeAsRef` associated type instead of a plain reference.
    fn as_ref(&self) -> bool {
        self.as_ref.is_some()
    }
}

impl Parse for MemoizeAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            as_ref: if input.is_empty() {
                None
            } else {
                Some(input.parse()?)
            },
        })
    }
}

pub struct MemoizeItem {
    item: ItemFn,
}

impl Parse for MemoizeItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let item: ItemFn = input.parse()?;
        let mut has_cx = false;
        for arg in &item.sig.inputs {
            match arg {
                FnArg::Receiver(r) => {
                    return Err(syn::Error::new_spanned(
                        r,
                        "memoize functions cannot take a `self` receiver",
                    ));
                }
                FnArg::Typed(pat_type) => {
                    if let Pat::Ident(pi) = &*pat_type.pat
                        && pi.ident == "cx"
                    {
                        has_cx = true;
                    }
                }
            }
        }
        if !has_cx {
            return Err(syn::Error::new_spanned(
                &item.sig,
                "memoize functions must take a `cx: &Cx` parameter",
            ));
        }
        Ok(Self { item })
    }
}

pub struct Memoize(MemoizeAttr, MemoizeItem);

impl Memoize {
    #[must_use]
    pub fn new(attr: MemoizeAttr, item: MemoizeItem) -> Self {
        Self(attr, item)
    }

    /// Parses the attribute and item streams into a [`Memoize`].
    ///
    /// # Errors
    ///
    /// Returns an error if either stream fails to parse as a valid memoize
    /// attribute or memoized function item.
    pub fn parse(attr: TokenStream, item: TokenStream) -> syn::Result<Self> {
        Ok(Self::new(syn::parse2(attr)?, syn::parse2(item)?))
    }
}

impl ToTokens for Memoize {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let item = &self.1.item;
        let attrs = &item.attrs;
        let vis = &item.vis;
        let sig = &item.sig;
        let ident = &sig.ident;
        let asyncness = &sig.asyncness;
        let body_stmts = &item.block.stmts;

        let mut generics = sig.generics.clone();
        generics.params.insert(0, parse_quote! { '__cx });
        let where_clause = &generics.where_clause;

        let mut new_inputs: Vec<TokenStream> = Vec::new();
        let mut key_idents: Vec<syn::Ident> = Vec::new();
        let mut borrowed_keys: Vec<TokenStream> = Vec::new();
        let mut closure_pats: Vec<TokenStream> = Vec::new();
        let mut destructures: Vec<TokenStream> = Vec::new();

        for arg in &sig.inputs {
            let FnArg::Typed(pat_type) = arg else {
                continue;
            };
            if let Pat::Ident(pi) = &*pat_type.pat
                && pi.ident == "cx"
            {
                let span = pi.span();
                new_inputs.push(quote_spanned! {span=> cx: &'__cx #topcoat_context::Cx });
                continue;
            }
            let ty = &pat_type.ty;
            let pat = &pat_type.pat;
            let is_ref = matches!(&**ty, syn::Type::Reference(_));
            let ident = if let Pat::Ident(pi) = &**pat
                && pi.by_ref.is_none()
                && pi.subpat.is_none()
            {
                let mutability = &pi.mutability;
                let ident = pi.ident.clone();
                new_inputs.push(quote! { #mutability #ident: #ty });
                closure_pats.push(quote! { #mutability #ident });
                ident
            } else {
                let synth = format_ident!("__key_{}", key_idents.len());
                new_inputs.push(quote! { #synth: #ty });
                destructures.push(quote! { let #pat = #synth; });
                closure_pats.push(quote! { #synth });
                synth
            };
            if is_ref {
                borrowed_keys.push(quote! { #ident });
            } else {
                borrowed_keys.push(quote! { &#ident });
            }
            key_idents.push(ident);
        }

        let return_ty = match &sig.output {
            ReturnType::Default => parse_quote! { () },
            ReturnType::Type(_, ty) => (**ty).clone(),
        };
        let return_type = quote! { #return_ty };
        let as_ref = self.0.as_ref();
        let output_type = if as_ref {
            quote! { <#return_type as #topcoat_context::MemoizeAsRef>::AsRef<'__cx> }
        } else {
            quote! { &'__cx #return_type }
        };

        let call = if asyncness.is_some() {
            quote! {
                #topcoat_context::memoize_cache(cx).memoize_async(
                    cx,
                    (#(#borrowed_keys,)*),
                    (#(#key_idents,)*),
                    async |cx, (#(#closure_pats,)*)| {
                        #(#destructures)*
                        #(#body_stmts)*
                    },
                ).await
            }
        } else {
            quote! {
                #topcoat_context::memoize_cache(cx).memoize(
                    cx,
                    (#(#borrowed_keys,)*),
                    (#(#key_idents,)*),
                    |cx, (#(#closure_pats,)*)| {
                        #(#destructures)*
                        #(#body_stmts)*
                    },
                )
            }
        };

        let output = if as_ref {
            quote! { #topcoat_context::MemoizeAsRef::as_ref(#call) }
        } else {
            call
        };

        quote! {
            #(#attrs)*
            #vis #asyncness fn #ident #generics (#(#new_inputs,)*) -> #output_type
            #where_clause
            {
                #output
            }
        }
        .to_tokens(tokens);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_empty_arguments() {
        let attr: MemoizeAttr = syn::parse_str("").unwrap();
        assert!(!attr.as_ref());
    }

    #[test]
    fn parses_as_ref() {
        let attr: MemoizeAttr = syn::parse_str("as_ref").unwrap();
        assert!(attr.as_ref());
    }

    #[test]
    fn rejects_unknown_argument() {
        assert!(syn::parse_str::<MemoizeAttr>("cloned").is_err());
    }
}
