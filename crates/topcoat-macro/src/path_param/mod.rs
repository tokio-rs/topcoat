use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    Ident, Path, Token, Visibility,
    parse::{Parse, ParseStream},
};
use topcoat_view::ast::ParseOption;

pub struct PathParam {
    vis: Visibility,
    name: Ident,
    ty: Option<PathParamType>,
    fn_name: Option<PathParamFnName>,
}

impl Parse for PathParam {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            vis: input.parse()?,
            name: input.parse()?,
            ty: input.call(PathParamType::parse_option)?,
            fn_name: input.call(PathParamFnName::parse_option)?,
        })
    }
}

struct PathParamFnName {
    _as_token: Token![as],
    name: Ident,
}

impl Parse for PathParamFnName {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            _as_token: input.parse()?,
            name: input.parse()?,
        })
    }
}

impl ParseOption for PathParamFnName {
    fn peek(input: ParseStream) -> bool {
        input.peek(Token![as])
    }
}

struct PathParamType {
    _colon_token: Token![:],
    path: Path,
}

impl Parse for PathParamType {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            _colon_token: input.parse()?,
            path: input.parse()?,
        })
    }
}

impl ParseOption for PathParamType {
    fn peek(input: ParseStream) -> bool {
        input.peek(Token![:])
    }
}

impl ToTokens for PathParam {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let vis = &self.vis;
        let name_string = self.name.to_string();
        let fn_name = self
            .fn_name
            .as_ref()
            .map(|fn_name| &fn_name.name)
            .unwrap_or(&self.name);

        let panic = quote! {
            panic!("path parameter \"{}\" was not found in request path", #name_string);
        };

        if let Some(ty) = &self.ty {
            let ty = &ty.path;
            quote! {
                #[::topcoat::context::memoize]
                #vis fn #fn_name(cx: &::topcoat::context::Cx) -> #ty {
                    for (key, value) in ::topcoat::context::raw_path_params(cx) {
                        if key == #name_string {
                            return str::parse::<#ty>(value).unwrap();
                        }
                    }
                    #panic
                }
            }
            .to_tokens(tokens);
        } else {
            quote! {
                #vis fn #fn_name(cx: &::topcoat::context::Cx) -> &str {
                    for (key, value) in ::topcoat::context::raw_path_params(cx) {
                        if key == #name_string {
                            return value;
                        }
                    }
                    #panic
                }
            }
            .to_tokens(tokens);
        }
    }
}
