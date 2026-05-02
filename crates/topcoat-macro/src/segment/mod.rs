use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{
    Ident, LitStr, Path, Token,
    parse::{Parse, ParseStream},
};
use topcoat_view::ast::ParseOption;

pub enum Segment {
    Static {
        name: LitStr,
    },
    Group {
        _underscore: Token![_],
    },
    Param {
        name: Ident,
        ty: Option<ParamType>,
        fn_name: Option<ParamFnName>,
    },
    CatchAll {
        _dotdot: Token![..],
        name: Ident,
    },
}

pub struct ParamType {
    _colon: Token![:],
    path: Path,
}

impl Parse for ParamType {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            _colon: input.parse()?,
            path: input.parse()?,
        })
    }
}

impl ParseOption for ParamType {
    fn peek(input: ParseStream) -> bool {
        input.peek(Token![:])
    }
}

pub struct ParamFnName {
    _as_token: Token![as],
    name: Ident,
}

impl Parse for ParamFnName {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            _as_token: input.parse()?,
            name: input.parse()?,
        })
    }
}

impl ParseOption for ParamFnName {
    fn peek(input: ParseStream) -> bool {
        input.peek(Token![as])
    }
}

impl Parse for Segment {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(LitStr) {
            Ok(Self::Static {
                name: input.parse()?,
            })
        } else if lookahead.peek(Token![_]) {
            Ok(Self::Group {
                _underscore: input.parse()?,
            })
        } else if lookahead.peek(Token![..]) {
            Ok(Self::CatchAll {
                _dotdot: input.parse()?,
                name: input.parse()?,
            })
        } else if lookahead.peek(Ident) {
            Ok(Self::Param {
                name: input.parse()?,
                ty: input.call(ParamType::parse_option)?,
                fn_name: input.call(ParamFnName::parse_option)?,
            })
        } else {
            Err(lookahead.error())
        }
    }
}

impl Segment {
    fn kind_ident(&self) -> Ident {
        let kind = match self {
            Self::Static { .. } => "Static",
            Self::Group { .. } => "Group",
            Self::Param { .. } => "Param",
            Self::CatchAll { .. } => "CatchAll",
        };
        Ident::new(kind, Span::call_site())
    }

    fn rename_tokens(&self) -> TokenStream {
        let lit = match self {
            Self::Static { name } => quote! { #name },
            Self::Group { .. } => return quote! { ::core::option::Option::None },
            Self::Param { name, .. } | Self::CatchAll { name, .. } => {
                let s = name.to_string();
                quote! { #s }
            }
        };
        quote! {
            ::core::option::Option::Some(::std::borrow::Cow::Borrowed(#lit))
        }
    }
}

impl ToTokens for Segment {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        if let Self::Param { name, ty, fn_name } = self {
            let ty_tokens = ty.as_ref().map(|ParamType { path, .. }| quote! { : #path });
            let fn_name_tokens = fn_name
                .as_ref()
                .map(|ParamFnName { name, .. }| quote! { as #name });
            quote! {
                ::topcoat::router::path_param!(#name #ty_tokens #fn_name_tokens);
            }
            .to_tokens(tokens);
        }

        if cfg!(feature = "discover") {
            let kind = self.kind_ident();
            let rename = self.rename_tokens();
            quote! {
                ::topcoat::inventory::submit! {
                    ::topcoat::router::Segment::new(
                        module_path!(),
                        ::core::option::Option::Some(::topcoat::router::SegmentKind::#kind),
                        #rename,
                    )
                }
            }
            .to_tokens(tokens);
        }
    }
}
