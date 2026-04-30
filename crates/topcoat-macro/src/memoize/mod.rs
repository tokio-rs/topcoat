use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{
    FnArg, ItemFn, Pat, ReturnType,
    parse::{Parse, ParseStream},
    parse_quote,
};

pub struct MemoizeAttr {}

impl Parse for MemoizeAttr {
    fn parse(_input: ParseStream) -> syn::Result<Self> {
        Ok(Self {})
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
    pub fn new(attr: MemoizeAttr, item: MemoizeItem) -> Self {
        Self(attr, item)
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
        let mut destructures: Vec<TokenStream> = Vec::new();

        for arg in &sig.inputs {
            let FnArg::Typed(pat_type) = arg else {
                continue;
            };
            if let Pat::Ident(pi) = &*pat_type.pat
                && pi.ident == "cx"
            {
                new_inputs.push(quote! { cx: &'__cx ::topcoat::context::Cx });
                continue;
            }
            let synth = format_ident!("__key_{}", key_idents.len());
            let ty = &pat_type.ty;
            let pat = &pat_type.pat;
            new_inputs.push(quote! { #synth: #ty });
            destructures.push(quote! { let #pat = #synth; });
            key_idents.push(synth);
        }

        let return_type = match &sig.output {
            ReturnType::Default => quote! { () },
            ReturnType::Type(_, ty) => quote! { #ty },
        };

        let call = if asyncness.is_some() {
            quote! {
                ::topcoat::context::memoize_raw_async(
                    cx,
                    (::std::marker::PhantomData::<__MemoizeTag>, #(#key_idents,)*),
                    async |(_, #(#key_idents,)*)| {
                        #(#destructures)*
                        #(#body_stmts)*
                    },
                ).await
            }
        } else {
            quote! {
                ::topcoat::context::memoize_raw(
                    cx,
                    (::std::marker::PhantomData::<__MemoizeTag>, #(#key_idents,)*),
                    |(_, #(#key_idents,)*)| {
                        #(#destructures)*
                        #(#body_stmts)*
                    },
                )
            }
        };

        quote! {
            #(#attrs)*
            #vis #asyncness fn #ident #generics (#(#new_inputs,)*)
                -> ::topcoat::context::Memoized<'__cx, #return_type>
            #where_clause
            {
                struct __MemoizeTag;
                #call
            }
        }
        .to_tokens(tokens);
    }
}
