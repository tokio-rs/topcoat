use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    FnArg, ItemFn, Pat,
    parse::{Parse, ParseStream},
    spanned::Spanned,
};

pub struct ComponentAttr {}

impl Parse for ComponentAttr {
    fn parse(_input: ParseStream) -> syn::Result<Self> {
        Ok(Self {})
    }
}

pub struct ComponentItem {
    item: ItemFn,
}

impl Parse for ComponentItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let item: ItemFn = input.parse()?;
        if item.sig.asyncness.is_none() {
            return Err(syn::Error::new(
                item.sig.fn_token.span(),
                "components must be async",
            ));
        }
        Ok(Self { item })
    }
}

impl ToTokens for ComponentItem {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let item = &self.item;
        let vis = &item.vis;
        let ident = &item.sig.ident;

        let generics = item.sig.generics.clone();
        // generics.params.insert(0, syn::parse_quote!('__implicit));
        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

        let mut has_cx = false;
        let mut fields = Vec::new();
        let mut args = Vec::new();

        for input in self.item.sig.inputs.iter() {
            match input {
                FnArg::Receiver(_) => panic!("component macro must not be used on methods"),
                FnArg::Typed(pat_type) => {
                    let ty = &pat_type.ty;
                    match &*pat_type.pat {
                        Pat::Ident(pi) if pi.ident == "cx" => {
                            has_cx = true;
                            args.push(quote! { cx });
                        }
                        Pat::Ident(pi) => {
                            fields.push(quote! { #pi: #ty });
                            args.push(quote! { self.#pi });
                        }
                        _ => panic!("function args must have an identifier"),
                    }
                }
            }
        }

        let body = if has_cx {
            quote! {
                #item
                ::topcoat::router::with_context(async |cx| #ident(#(#args),*).await).await
            }
        } else {
            quote! {
                #item
                #ident(#(#args),*).await
            }
        };

        quote! {
            #[allow(non_camel_case_types)]
            #vis struct #ident #impl_generics #where_clause {
                #(#vis #fields),*
            }

            impl #impl_generics ::topcoat::component::Component for #ident #ty_generics #where_clause {
                async fn render(self) -> ::topcoat::view::View {
                    #body
                }
            }
        }
        .to_tokens(tokens);
    }
}
