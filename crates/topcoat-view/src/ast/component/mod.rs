mod attr;
mod item;

pub use attr::*;
pub use item::*;

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    FnArg, Lifetime, Pat, ReturnType, TypeReference, parse_quote,
    visit_mut::{self, VisitMut},
};

use crate::ast::component::{ComponentAttr, ComponentItem};

pub struct Component {
    _attr: ComponentAttr,
    item: ComponentItem,
}

impl Component {
    pub fn new(attr: ComponentAttr, item: ComponentItem) -> Self {
        Self { _attr: attr, item }
    }

    pub fn parse(attr: TokenStream, item: TokenStream) -> syn::Result<Self> {
        Ok(Self::new(syn::parse2(attr)?, syn::parse2(item)?))
    }
}

impl ToTokens for Component {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let mut item = self.item.item().clone();
        let mut generics = item.sig.generics.clone();
        item.sig
            .inputs
            .insert(0, parse_quote! { __cx: &::topcoat::context::Cx });
        item.sig
            .inputs
            .insert(1, parse_quote! { __f: &mut ::topcoat::view::Formatter<'_> });

        let vis = &item.vis;
        let ident = &item.sig.ident;
        let ReturnType::Type(_, return_ty) = &item.sig.output else {
            unreachable!("validated in Parse");
        };

        let mut fields = Vec::new();
        let mut destructure = Vec::new();
        let mut visitor = ImplicitLifetimeVisitor { used: false };

        for input in self.item.item().sig.inputs.iter() {
            let FnArg::Typed(pat_type) = input else {
                unreachable!("validated in Parse");
            };
            let Pat::Ident(pi) = &*pat_type.pat else {
                unreachable!("validated in Parse");
            };
            if pi.ident == "cx" {
                destructure.push(quote! { let #pi = __cx; });
            } else {
                let mut ty = (*pat_type.ty).clone();
                visitor.visit_type_mut(&mut ty);
                fields.push(quote! { #pi: #ty });
                destructure.push(quote! { let #pi = self.#pi; });
            }
        }

        if visitor.used {
            generics.params.insert(0, parse_quote! { '__implicit });
        }
        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

        let block = item.block;

        quote! {
            #[allow(non_camel_case_types)]
            #vis struct #ident #impl_generics #where_clause {
                #(#vis #fields),*
            }

            impl #impl_generics ::topcoat::view::Component for #ident #ty_generics #where_clause {
                type Error = <#return_ty as ::topcoat::internal::ResultExt>::E;

                async fn render(self) -> #return_ty {
                    let cx = self.__cx;
                    #block
                }
            }
        }
        .to_tokens(tokens);
    }
}

struct ImplicitLifetimeVisitor {
    used: bool,
}

impl VisitMut for ImplicitLifetimeVisitor {
    fn visit_lifetime_mut(&mut self, lt: &mut Lifetime) {
        if lt.ident == "_" {
            *lt = parse_quote! { '__implicit };
            self.used = true;
        }
    }

    fn visit_type_reference_mut(&mut self, tr: &mut TypeReference) {
        if tr.lifetime.is_none() {
            tr.lifetime = Some(parse_quote! { '__implicit });
            self.used = true;
        }
        visit_mut::visit_type_reference_mut(self, tr);
    }
}
