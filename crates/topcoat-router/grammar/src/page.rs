use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    ItemFn, LitStr, ReturnType,
    parse::{Parse, ParseStream},
    spanned::Spanned,
};
use topcoat_core_grammar::{
    ParseOption,
    paths::{topcoat_inventory, topcoat_router, topcoat_view, topcoat_view_macro},
};

use super::{
    common::{HandlerArg, HandlerArgs, request_ident},
    method::Methods,
};

pub struct PageAttr {
    /// The declared HTTP methods; the page serves `GET` when omitted.
    methods: Option<Methods>,
    path: Option<LitStr>,
}

impl Parse for PageAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            methods: Methods::parse_option(input)?,
            path: input.peek(LitStr).then(|| input.parse()).transpose()?,
        })
    }
}

pub struct PageItem {
    item: ItemFn,
    args: HandlerArgs,
}

impl Parse for PageItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let item: ItemFn = input.parse()?;
        if item.sig.asyncness.is_none() {
            return Err(syn::Error::new(
                item.sig.fn_token.span(),
                "page functions must be async",
            ));
        }
        if let ReturnType::Default = &item.sig.output {
            return Err(syn::Error::new(
                item.sig.fn_token.span(),
                "page functions must declare a return type",
            ));
        }
        let args = HandlerArgs::parse(&item, "page")?;
        Ok(Self { item, args })
    }
}

pub struct Page(PageAttr, PageItem);

impl Page {
    #[must_use]
    pub fn new(attr: PageAttr, item: PageItem) -> Self {
        Self(attr, item)
    }

    /// Parses a page attribute and item from token streams.
    ///
    /// # Errors
    ///
    /// Returns an error if either token stream fails to parse as a
    /// [`PageAttr`] or [`PageItem`], or if the item is not a valid page
    /// handler.
    pub fn parse(attr: TokenStream, item: TokenStream) -> syn::Result<Self> {
        Ok(Self::new(syn::parse2(attr)?, syn::parse2(item)?))
    }
}

impl ToTokens for Page {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let attr = &self.0;
        let item = &self.1.item;
        let args = &self.1.args;
        let vis = &item.vis;
        let ident = &item.sig.ident;
        let output = &item.sig.output;

        // Component face: the user's body becomes the component body, so its
        // tail `view!` compiles onto the live render. It takes `cx` when the
        // page declares it, and a page that reads a request body takes the
        // already-parsed value as a `body` prop, rebound to the declared
        // pattern. The marker struct this expands to is a unit struct, so
        // `#ident` stays a value usable directly in `router.page(...)`.
        // The context parameter is re-emitted as the user wrote it, so the
        // type path they imported stays used.
        let cx_param = args
            .iter()
            .zip(&item.sig.inputs)
            .find_map(|(arg, input)| matches!(arg, HandlerArg::Cx).then(|| quote! { #input, }));
        let body_param = args.request().map(|ty| quote! { body: #ty, });
        let rebind_body = args.request_pat().and_then(|pat| match pat {
            syn::Pat::Ident(pat) if pat.ident == "body" => None,
            pat => Some(quote! { let #pat = body; }),
        });
        // The body's statements are spliced directly, not as a nested block,
        // so a direct `let x = view! { ... };` stays a direct statement of
        // the component body.
        let stmts = &item.block.stmts;
        let attrs = &item.attrs;
        quote! {
            #(#attrs)*
            #[#topcoat_view_macro::component]
            #vis async fn #ident(#cx_param #body_param) #output {
                #rebind_body
                #(#stmts)*
            }
        }
        .to_tokens(tokens);

        // The render function backing the registered page: it parses the
        // request body (when the page takes one) and starts the component's
        // render with the fill the router minted.
        let parse_request = args.request().map(|request_ty| {
            let request_ident = request_ident();
            quote! {
                let #request_ident = <#request_ty as #topcoat_router::request::FromRequest>::from_request(cx, body).await?;
            }
        });
        let body_setter = args.request().map(|_| {
            let request_ident = request_ident();
            quote! { .body(#request_ident) }
        });
        let render = quote! {
            |cx, body, fill| ::std::boxed::Box::pin(async move {
                use #topcoat_view::Component;
                #parse_request
                let props = #ident::props_builder()#body_setter.build();
                #[allow(clippy::default_constructed_unit_structs)]
                Component::render(#ident::default(), cx, props, fill).await
            })
        };

        // The erased page is built once in a `const` so it can be used from
        // both the `From` impl (backing manual `router.page(#ident)`
        // registration) and the discovery submission (which expands to a
        // `static`, requiring a const initializer). It is named after the page
        // so the render closure carries that name in backtraces and profiles.
        let methods = attr.methods.as_ref().map_or_else(
            || quote! { #topcoat_router::OwnedMethods::One(#topcoat_router::Method::GET) },
            ToTokens::to_token_stream,
        );
        let erased = if let Some(path) = attr.path.as_ref() {
            quote! {
                #[allow(non_upper_case_globals)]
                const #ident: #topcoat_router::PageFn = #topcoat_router::PageFn::const_new(
                    #methods,
                    ::std::borrow::Cow::Borrowed(#topcoat_router::Path::new(#path)),
                    #render,
                );

                impl ::core::convert::From<#ident> for #topcoat_router::PageFn {
                    fn from(_: #ident) -> Self {
                        #ident
                    }
                }
            }
        } else {
            quote! {
                #[allow(non_upper_case_globals)]
                const #ident: #topcoat_router::ModulePageFn =
                    #topcoat_router::ModulePageFn::new(#methods, module_path!(), #render);

                impl ::core::convert::From<#ident> for #topcoat_router::ModulePageFn {
                    fn from(_: #ident) -> Self {
                        #ident
                    }
                }
            }
        };

        let submit =
            cfg!(feature = "discover").then(|| quote! { #topcoat_inventory::submit! { #ident } });

        quote! {
            const _: () = {
                #erased

                #submit
            };
        }
        .to_tokens(tokens);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_err(source: &str) -> String {
        match syn::parse_str::<PageItem>(source) {
            Ok(_) => panic!("expected parse error for `{source}`"),
            Err(err) => err.to_string(),
        }
    }

    #[test]
    fn attr_without_methods_leaves_them_unset() {
        let attr: PageAttr = syn::parse_str("\"/about\"").unwrap();
        assert!(attr.methods.is_none());
        assert!(attr.path.is_some());

        let attr: PageAttr = syn::parse_str("").unwrap();
        assert!(attr.methods.is_none());
        assert!(attr.path.is_none());
    }

    #[test]
    fn attr_accepts_methods_before_the_path() {
        let attr: PageAttr = syn::parse_str("POST \"/submit\"").unwrap();
        assert!(attr.methods.is_some());
        assert!(attr.path.is_some());
    }

    #[test]
    fn attr_accepts_methods_without_a_path() {
        for source in ["POST", "[GET, POST]", "*"] {
            let attr: PageAttr = syn::parse_str(source).unwrap();
            assert!(attr.methods.is_some());
            assert!(attr.path.is_none());
        }
    }

    #[test]
    fn accepts_async_fn_with_return_type() {
        syn::parse_str::<PageItem>("async fn home(cx: &Cx) -> Result { todo!() }").unwrap();
    }

    #[test]
    fn accepts_a_destructured_request_parameter() {
        syn::parse_str::<PageItem>(
            "async fn search(Form(input): Form<Search>, cx: &Cx) -> Result { todo!() }",
        )
        .unwrap();
    }

    #[test]
    fn rejects_non_async_fn() {
        assert!(parse_err("fn home() -> Result { todo!() }").contains("must be async"));
    }

    #[test]
    fn rejects_missing_return_type() {
        assert!(parse_err("async fn home() {}").contains("must declare a return type"));
    }

    #[test]
    fn rejects_self_receiver() {
        let err = parse_err("async fn home(&self) -> Result { todo!() }");
        assert!(err.contains("cannot take a `self` receiver"));
    }

    #[test]
    fn rejects_multiple_request_parameters() {
        let err = parse_err("async fn home(a: Form<A>, b: Form<B>) -> Result { todo!() }");
        assert!(err.contains("more than one request body parameter"));
    }
}
