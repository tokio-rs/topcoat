mod attr;
mod item;

pub use attr::*;
pub use item::*;

use heck::ToPascalCase;
use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{
    Attribute, FnArg, GenericParam, Lifetime, Pat, ReturnType, TypeReference,
    ext::IdentExt,
    parse_quote,
    visit_mut::{self, VisitMut},
};

use crate::ast::component::{ComponentAttr, ComponentItem};

/// A parsed `#[component] async fn ...`. Expands into:
///
/// - a props struct named after the function in PascalCase plus `Props`
///   (`button` becomes `ButtonProps`), deriving [`Props`] so it gets a
///   typestate builder. `#[default]` and `#[into]` on function parameters are
///   forwarded to the corresponding props fields. A `child` parameter is
///   always `#[default]`, so calls without children fall back to an empty
///   view.
/// - a zero-sized marker struct named after the function that implements
///   [`topcoat::view::Component`] with a `render` method calling the original
///   function body.
///
/// [`Props`]: derive.Props.html
/// [`topcoat::view::Component`]: trait.Component.html
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
        item.sig.generics.params.insert(0, parse_quote! { '__cx });
        item.sig
            .inputs
            .insert(0, parse_quote! { __cx: &'__cx ::topcoat::context::Cx });

        // `#[default]` / `#[into]` belong to the generated props struct; they
        // are not valid attributes on the re-emitted function's parameters.
        for input in item.sig.inputs.iter_mut() {
            if let FnArg::Typed(pat_type) = input {
                pat_type.attrs.retain(|attr| !is_props_attr(attr));
            }
        }

        let vis = &item.vis;
        let ident = &item.sig.ident;
        let props_ident = format_ident!(
            "{}Props",
            ident.unraw().to_string().to_pascal_case(),
            span = ident.span()
        );
        let ReturnType::Type(_, return_ty) = &item.sig.output else {
            unreachable!("validated in Parse");
        };

        let mut fields = Vec::new();
        let mut args = Vec::new();
        let mut visitor = ImplicitLifetimeVisitor { used: false };

        for input in self.item.item().sig.inputs.iter() {
            let FnArg::Typed(pat_type) = input else {
                unreachable!("validated in Parse");
            };
            let Pat::Ident(pi) = &*pat_type.pat else {
                unreachable!("validated in Parse");
            };
            if pi.ident == "cx" {
                args.push(quote! { cx });
            } else {
                let mut ty = (*pat_type.ty).clone();
                visitor.visit_type_mut(&mut ty);

                let attrs: Vec<&Attribute> = pat_type
                    .attrs
                    .iter()
                    .filter(|attr| is_props_attr(attr))
                    .collect();
                let child_default = (pi.ident == "child"
                    && !attrs.iter().any(|attr| attr.path().is_ident("default")))
                .then(|| quote! { #[default] });

                let field_ident = &pi.ident;
                fields.push(quote! { #(#attrs)* #child_default #vis #field_ident: #ty });
                args.push(quote! { props.#field_ident });
            }
        }

        if visitor.used {
            generics.params.insert(0, parse_quote! { '__implicit });
        }
        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

        let phantom_args = generics.params.iter().filter_map(|param| match param {
            GenericParam::Lifetime(param) => {
                let lifetime = &param.lifetime;
                Some(quote! { &#lifetime () })
            }
            GenericParam::Type(param) => {
                let ident = &param.ident;
                Some(quote! { #ident })
            }
            GenericParam::Const(_) => None,
        });

        let props_pat = if fields.is_empty() {
            quote! { _props }
        } else {
            quote! { props }
        };

        let body = quote! {
            #item
            #ident(cx, #(#args),*).await
        };

        quote! {
            #[derive(::topcoat::view::Props)]
            #vis struct #props_ident #impl_generics #where_clause {
                #(#fields),*
            }

            #[allow(non_camel_case_types)]
            #vis struct #ident #impl_generics (
                #vis ::core::marker::PhantomData<(#(#phantom_args,)*)>,
            ) #where_clause;

            impl #impl_generics ::topcoat::view::Component for #ident #ty_generics #where_clause {
                type Props = #props_ident #ty_generics;

                async fn render(self, cx: &::topcoat::context::Cx, #props_pat: Self::Props) -> #return_ty {
                    let __cx = cx;
                    #body
                }
            }
        }
        .to_tokens(tokens);
    }
}

fn is_props_attr(attr: &Attribute) -> bool {
    attr.path().is_ident("default") || attr.path().is_ident("into")
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
