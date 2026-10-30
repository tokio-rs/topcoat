use proc_macro2::TokenStream;
use quote::{ToTokens, quote, quote_spanned};
use syn::{
    FnArg, GenericParam, ItemFn, Pat, ReturnType, Token,
    parse::{Parse, ParseStream},
    parse_quote,
    punctuated::Punctuated,
    spanned::Spanned,
};
use topcoat_core_grammar::{
    ParseOption,
    paths::{topcoat_context, topcoat_inventory, topcoat_router, topcoat_view, topcoat_view_macro},
};

use super::{
    common::{HandlerArg, HandlerArgs, HandlerPath, RouteContext},
    method::Methods,
};

pub struct PageAttr {
    /// The declared HTTP methods; the page serves `GET` when omitted.
    pub methods: Option<Methods>,
    pub path: Option<HandlerPath>,
    /// The comma between the methods or path and the `with(...)` entries.
    pub comma: Option<Token![,]>,
    pub context: Punctuated<RouteContext, Token![,]>,
}

impl Parse for PageAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let methods = Methods::parse_option(input)?;
        let path = input.call(HandlerPath::parse_option)?;
        Ok(Self {
            comma: if (methods.is_some() || path.is_some()) && !input.is_empty() {
                Some(input.parse()?)
            } else {
                None
            },
            methods,
            path,
            context: Punctuated::parse_terminated(input)?,
        })
    }
}

/// The annotated `async fn` that becomes a page: a component taking,
/// optionally, the request context as `cx` and the request body as one
/// parameter of any pattern, such as `Form(input): Form<T>`.
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

        if let Some(lifetime) = item.sig.generics.lifetimes().next() {
            return Err(syn::Error::new_spanned(
                lifetime,
                "page functions cannot declare lifetime parameters; use elided lifetimes for request borrows",
            ));
        }

        let args = HandlerArgs::parse(&item, "page")?;
        for (arg, input) in args.iter().zip(&item.sig.inputs) {
            let (HandlerArg::Cx, FnArg::Typed(pat_type)) = (arg, input) else {
                continue;
            };
            if !matches!(&*pat_type.pat, Pat::Ident(pat) if pat.ident == "cx") {
                return Err(syn::Error::new_spanned(
                    pat_type,
                    "the request context parameter must be named `cx`",
                ));
            }
        }

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
    /// function.
    pub fn parse(attr: TokenStream, item: TokenStream) -> syn::Result<Self> {
        Ok(Self::new(syn::parse2(attr)?, syn::parse2(item)?))
    }
}

impl ToTokens for Page {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let attr = &self.0;
        let item = &self.1.item;
        let args = &self.1.args;
        let ident = &item.sig.ident;
        let is_generic = item
            .sig
            .generics
            .params
            .iter()
            .any(|param| matches!(param, GenericParam::Type(_) | GenericParam::Const(_)));
        let mut generics = item.sig.generics.clone();
        if is_generic {
            // Page/ModulePage are Send + Sync + 'static. HrefTarget needs the
            // same bound so it looks up the identical specialization.
            generics.make_where_clause().predicates.push(parse_quote! {
                Self: ::core::marker::Send + ::core::marker::Sync + 'static
            });
        }
        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

        // Rust statics inside generic functions are shared by all
        // monomorphizations. Key the IDs by the complete marker type instead
        // so home<A> and home<B> cannot resolve each other's links.
        let (identity_static, identity) = if is_generic {
            (
                quote! {
                    static IDS: ::std::sync::LazyLock<
                        ::std::sync::Mutex<::std::collections::HashMap<::core::any::TypeId, #topcoat_router::RouteId>>
                    > = ::std::sync::LazyLock::new(|| ::std::sync::Mutex::new(::std::collections::HashMap::new()));
                },
                quote! {{
                    let mut ids = IDS.lock().unwrap_or_else(::std::sync::PoisonError::into_inner);
                    *ids.entry(::core::any::TypeId::of::<Self>())
                        .or_insert_with(#topcoat_router::RouteId::new)
                }},
            )
        } else {
            (
                quote! {
                    static ID: ::std::sync::LazyLock<#topcoat_router::RouteId> =
                        ::std::sync::LazyLock::new(#topcoat_router::RouteId::new);
                },
                quote! { *ID },
            )
        };

        // The component takes the request body as its `body` prop, so a body
        // parameter bound to another pattern is rebound from `body` inside.
        let mut face = item.clone();
        for (arg, input) in args.iter().zip(&mut face.sig.inputs) {
            if let (HandlerArg::Request(_), FnArg::Typed(pat_type)) = (arg, input)
                && !matches!(&*pat_type.pat, Pat::Ident(pat) if pat.ident == "body")
            {
                let pat = std::mem::replace(&mut *pat_type.pat, parse_quote! { body });
                face.block
                    .stmts
                    .insert(0, parse_quote! { let #pat = body; });
            }
        }
        let component = quote! {
            #[#topcoat_view_macro::component]
            #face
        };

        // A request that fails to parse becomes the error the view's stream
        // yields, exactly like an error from the page body.
        let (parse_request, body_prop) = match args.request() {
            Some(body_ty) => (
                quote_spanned! {body_ty.span()=>
                    let body = <#body_ty as #topcoat_router::request::FromRequest>::from_request(&cx, body).await?;
                },
                Some(quote! { .body(body) }),
            ),
            None => (quote! { ::core::mem::drop(body); }, None),
        };
        let render = quote! {
            fn render<'a>(
                &'a self,
                cx: &'a #topcoat_context::Cx,
                body: #topcoat_router::Body,
            ) -> #topcoat_view::BoxView<'a> {
                ::std::boxed::Box::pin(#topcoat_view::HoistView::new(
                    #topcoat_view::internal::ThenView::new(async move {
                        #parse_request
                        let props = <Self as #topcoat_view::Component>::props_builder()
                            #body_prop
                            .build();
                        <Self as #topcoat_view::Component>::render(
                            <Self as ::core::default::Default>::default(), cx, props,
                        )
                        .await
                    })
                ))
            }
        };
        let methods = attr.methods.as_ref().map_or_else(
            || quote! { #topcoat_router::Methods::Only(&[#topcoat_router::Method::GET]) },
            ToTokens::to_token_stream,
        );
        let context = RouteContext::context_method(&attr.context);
        let id = quote! {
            fn id(&self) -> #topcoat_router::RouteId {
                #identity
            }
        };
        let methods = quote! {
            fn methods(&self) -> #topcoat_router::Methods<'_> {
                const METHODS: #topcoat_router::Methods<'static> = #methods;
                METHODS
            }
        };
        let (page, submit_as) = if let Some(path) =
            attr.path.as_ref().and_then(HandlerPath::absolute)
        {
            let page = quote! {
                impl #impl_generics #topcoat_router::Page for #ident #ty_generics #where_clause {
                    #id

                    #methods

                    fn path(&self) -> &#topcoat_router::Path {
                        const PATH: &#topcoat_router::Path = #topcoat_router::Path::new(#path);
                        PATH
                    }

                    #context

                    #render
                }
            };
            (page, quote! { #topcoat_router::Page })
        } else {
            let relative_path = attr
                .path
                .as_ref()
                .and_then(HandlerPath::relative_path_method);
            let page = quote! {
                impl #impl_generics #topcoat_router::ModulePage for #ident #ty_generics #where_clause {
                    #id

                    #methods

                    fn module_path(&self) -> &'static str {
                        ::core::module_path!()
                    }

                    #relative_path

                    #context

                    #render
                }
            };
            (page, quote! { #topcoat_router::ModulePage })
        };

        // href! resolves the marker to the URL path it is served at, through
        // the router that dispatched the current request.
        let href_target = quote! {
            impl #impl_generics #topcoat_router::HrefTarget for #ident #ty_generics #where_clause {
                fn path<'cx>(&self, cx: &'cx #topcoat_context::Cx) -> &'cx #topcoat_router::Path {
                    match #topcoat_router::route_endpoint(cx, #identity) {
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

        // Discovery cannot choose type/const arguments. Generic pages are
        // registered explicitly with a concrete marker's Default instance.
        let submit = (cfg!(feature = "discover") && !is_generic).then(|| {
            quote! { #topcoat_inventory::submit! { &#ident as &'static dyn #submit_as } }
        });

        quote! {
            #component

            const _: () = {
                #identity_static

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
    fn attr_accepts_context_entries_after_any_prefix() {
        let cases = [
            ("with(a)", 1),
            ("with(a), with(b),", 2),
            ("\"/about\", with(a)", 1),
            ("POST, with(a)", 1),
            ("POST \"/submit\", with(a), with(b)", 2),
        ];
        for (source, entries) in cases {
            let attr: PageAttr = syn::parse_str(source).unwrap();
            assert_eq!(attr.context.len(), entries, "{source}");
        }
    }

    #[test]
    fn attr_does_not_take_with_for_a_method() {
        let attr: PageAttr = syn::parse_str("with(a)").unwrap();
        assert!(attr.methods.is_none());
        assert_eq!(attr.context.len(), 1);
    }

    #[test]
    fn attr_rejects_context_entries_without_a_separating_comma() {
        assert!(syn::parse_str::<PageAttr>("\"/about\" with(a)").is_err());
        assert!(syn::parse_str::<PageAttr>("with(a) with(b)").is_err());
    }

    #[test]
    fn attr_rejects_a_leading_comma() {
        assert!(syn::parse_str::<PageAttr>(", with(a)").is_err());
    }

    #[test]
    fn attr_rejects_unknown_entries() {
        assert!(syn::parse_str::<PageAttr>("\"/about\", title(a)").is_err());
    }

    #[test]
    fn attr_rejects_lowercase_methods() {
        assert!(syn::parse_str::<PageAttr>("get \"/about\"").is_err());
        assert!(syn::parse_str::<PageAttr>("[GET, post] \"/about\"").is_err());
    }

    #[test]
    fn accepts_async_fn_with_return_type() {
        let item = syn::parse_str::<PageItem>("async fn home() -> Result { todo!() }").unwrap();
        assert!(item.args.request().is_none());
    }

    #[test]
    fn accepts_a_body_parameter() {
        let item =
            syn::parse_str::<PageItem>("async fn search(body: Form<Search>) -> Result { todo!() }")
                .unwrap();
        assert!(item.args.request().is_some());
    }

    #[test]
    fn accepts_a_destructured_body_parameter() {
        let item = syn::parse_str::<PageItem>(
            "async fn search(Form(input): Form<Search>) -> Result { todo!() }",
        )
        .unwrap();
        assert!(item.args.request().is_some());
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
    fn rejects_named_lifetimes_with_a_clear_diagnostic() {
        let error = parse_err("async fn home<'a>(cx: &'a Cx) -> Result { todo!() }");
        assert!(error.contains("cannot declare lifetime parameters"));
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
    fn rejects_a_misnamed_context_parameter() {
        let err = parse_err("async fn home(context: &Cx) -> Result { todo!() }");
        assert!(err.contains("must be named `cx`"));
    }

    #[test]
    fn rejects_multiple_request_parameters() {
        let err = parse_err("async fn home(a: Form<A>, b: Form<B>) -> Result { todo!() }");
        assert!(err.contains("more than one request body parameter"));
    }
}
