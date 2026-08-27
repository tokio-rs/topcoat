use proc_macro2::TokenStream;
use quote::{ToTokens, quote, quote_spanned};
use syn::{
    FnArg, ItemFn, LitStr, Pat, ReturnType, Type,
    parse::{Parse, ParseStream},
    spanned::Spanned,
};
use topcoat_core_grammar::{
    ParseOption,
    paths::{topcoat_context, topcoat_inventory, topcoat_router, topcoat_view, topcoat_view_macro},
};

use super::method::Methods;

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

/// The annotated `async fn` that becomes a page: a component optionally
/// taking the request body as its `body` parameter and the request context
/// as `cx`.
pub struct PageItem {
    item: ItemFn,
}

impl PageItem {
    /// The declared type of the `body` parameter, if any.
    fn body(&self) -> Option<&Type> {
        self.item.sig.inputs.iter().find_map(|arg| match arg {
            FnArg::Typed(pat_type) if is_named(&pat_type.pat, "body") => Some(&*pat_type.ty),
            _ => None,
        })
    }
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

        let (mut body, mut cx) = (false, false);
        for arg in &item.sig.inputs {
            let FnArg::Typed(pat_type) = arg else {
                return Err(syn::Error::new_spanned(
                    arg,
                    "page functions cannot take a `self` receiver",
                ));
            };
            let seen = if is_named(&pat_type.pat, "body") {
                &mut body
            } else if is_named(&pat_type.pat, "cx") {
                &mut cx
            } else {
                &mut true
            };
            if *seen {
                return Err(syn::Error::new_spanned(
                    pat_type,
                    "page functions only accept an optional `body` request parameter and an optional `cx: &Cx` parameter",
                ));
            }
            *seen = true;
        }

        Ok(Self { item })
    }
}

/// Whether the pattern binds a plain identifier of the given name.
fn is_named(pat: &Pat, name: &str) -> bool {
    matches!(pat, Pat::Ident(pat) if pat.ident == name)
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
    /// function.
    pub fn parse(attr: TokenStream, item: TokenStream) -> syn::Result<Self> {
        Ok(Self::new(syn::parse2(attr)?, syn::parse2(item)?))
    }
}

impl ToTokens for Page {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let attr = &self.0;
        let item = &self.1.item;
        let ident = &item.sig.ident;

        let component = quote! {
            #[#topcoat_view_macro::component]
            #item
        };

        // A request that fails to parse becomes the error the view's stream
        // yields, exactly like an error from the page body.
        let (parse_request, body_prop) = match self.1.body() {
            Some(body_ty) => (
                quote_spanned! {body_ty.span()=>
                    let body = <#body_ty as #topcoat_router::request::FromRequest>::from_request(&cx, body).await?;
                },
                Some(quote! { .body(body) }),
            ),
            None => (quote! { ::core::mem::drop(body); }, None),
        };
        // The view owns copies of the request context and buffer it is
        // rendered with and drives the component's view in place, which
        // lets the view borrow them.
        let render = quote! {
            fn render(
                &self,
                cx: &#topcoat_context::Cx,
                buf: &#topcoat_view::ViewBuffer,
                body: #topcoat_router::Body,
            ) -> #topcoat_view::BoxView<'_> {
                let cx = cx.clone();
                let buf = buf.clone();
                ::std::boxed::Box::pin(#topcoat_view::internal::MoveView::new(async move {
                    #parse_request
                    let props = <#ident as #topcoat_view::Component>::props_builder()
                        #body_prop
                        .build();
                    let view = <#ident as #topcoat_view::Component>::render(
                        #ident, &cx, &buf, props,
                    )
                    .await?;
                    #topcoat_view::internal::drive(view).await
                }))
            }
        };
        let methods = attr.methods.as_ref().map_or_else(
            || quote! { #topcoat_router::Methods::Only(&[#topcoat_router::Method::GET]) },
            ToTokens::to_token_stream,
        );
        let id = quote! {
            fn id(&self) -> #topcoat_router::RouteId {
                *ID
            }
        };
        let methods = quote! {
            fn methods(&self) -> #topcoat_router::Methods<'_> {
                const METHODS: #topcoat_router::Methods<'static> = #methods;
                METHODS
            }
        };
        let (page, submit_as) = if let Some(path) = attr.path.as_ref() {
            let page = quote! {
                impl #topcoat_router::Page for #ident {
                    #id

                    #methods

                    fn path(&self) -> &#topcoat_router::Path {
                        const PATH: &#topcoat_router::Path = #topcoat_router::Path::new(#path);
                        PATH
                    }

                    #render
                }
            };
            (page, quote! { #topcoat_router::Page })
        } else {
            let page = quote! {
                impl #topcoat_router::ModulePage for #ident {
                    #id

                    #methods

                    fn module_path(&self) -> &'static str {
                        ::core::module_path!()
                    }

                    #render
                }
            };
            (page, quote! { #topcoat_router::ModulePage })
        };

        // href! resolves the marker to the URL path it is served at, through
        // the router that dispatched the current request.
        let href_target = quote! {
            impl #topcoat_router::HrefTarget for #ident {
                fn path<'cx>(&self, cx: &'cx #topcoat_context::Cx) -> &'cx #topcoat_router::Path {
                    match #topcoat_router::route_endpoint(cx, *ID) {
                        ::core::option::Option::Some(endpoint) => endpoint.path(),
                        ::core::option::Option::None => ::core::panic!(::core::concat!(
                            "page `",
                            ::core::stringify!(#ident),
                            "` is not registered on the router serving this request",
                        )),
                    }
                }
            }
        };

        // Discovery collects the marker erased behind its trait.
        let submit = cfg!(feature = "discover").then(|| {
            quote! { #topcoat_inventory::submit! { &#ident as &'static dyn #submit_as } }
        });

        quote! {
            #component

            const _: () = {
                static ID: ::std::sync::LazyLock<#topcoat_router::RouteId> =
                    ::std::sync::LazyLock::new(#topcoat_router::RouteId::new);

                #page

                #href_target

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
        let item = syn::parse_str::<PageItem>("async fn home() -> Result { todo!() }").unwrap();
        assert!(item.body().is_none());
    }

    #[test]
    fn accepts_a_body_parameter() {
        let item =
            syn::parse_str::<PageItem>("async fn search(body: Form<Search>) -> Result { todo!() }")
                .unwrap();
        assert!(item.body().is_some());
    }

    #[test]
    fn accepts_cx_and_body_in_any_order() {
        syn::parse_str::<PageItem>(
            "async fn search(cx: &Cx, body: Form<Search>) -> Result { todo!() }",
        )
        .unwrap();
        syn::parse_str::<PageItem>(
            "async fn search(body: Form<Search>, cx: &Cx) -> Result { todo!() }",
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
    fn rejects_unknown_parameter_names() {
        let err = parse_err("async fn home(input: Form<A>) -> Result { todo!() }");
        assert!(err.contains("only accept"));
    }

    #[test]
    fn rejects_destructured_parameters() {
        let err = parse_err("async fn home(Form(input): Form<A>) -> Result { todo!() }");
        assert!(err.contains("only accept"));
    }

    #[test]
    fn rejects_duplicate_body_parameters() {
        let err = parse_err("async fn home(body: Form<A>, body: Form<B>) -> Result { todo!() }");
        assert!(err.contains("only accept"));
    }
}
