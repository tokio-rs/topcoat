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
        let ident = &item.sig.ident;
        let generics = &item.sig.generics;
        // let props_struct_ident = Ident::new(
        //     &(self.item.sig.ident.to_string().to_upper_camel_case() + "Props"),
        //     self.item.sig.ident.span(),
        // );

        let fields = self.item.sig.inputs.iter().map(|input| match input {
            FnArg::Receiver(_) => panic!("component macro must not be used on methods"),
            FnArg::Typed(pat_type) => {
                let ty = &pat_type.ty;
                match &*pat_type.pat {
                    Pat::Ident(ident) => quote! { #ident: #ty },
                    _ => panic!("function args must have an identifier"),
                }
            }
        });

        let args = self.item.sig.inputs.iter().map(|input| match input {
            FnArg::Receiver(_) => panic!("component macro must not be used on methods"),
            FnArg::Typed(pat_type) => match &*pat_type.pat {
                Pat::Ident(ident) => quote! { self.#ident },
                _ => panic!("function args must have an identifier"),
            },
        });

        quote! {
            #[allow(non_camel_case_types)]
            struct #ident #generics {
                #(#fields),*
            }

            impl #generics ::topcoat::component::Component for #ident #generics {
                async fn render(self) -> ::topcoat::view::View {
                    #item
                    #ident(#(#args),*).await
                }
            }
        }
        .to_tokens(tokens);
    }
}
