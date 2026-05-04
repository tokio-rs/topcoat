use std::collections::HashSet;

use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{
    Ident, LitStr, Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

use crate::quote_option::QuoteOption;

pub struct Segment {
    attrs: Punctuated<SegmentAttr, Token![,]>,
}

impl Segment {
    fn find_kind(&self) -> Option<&Ident> {
        self.attrs.iter().find_map(SegmentAttr::as_kind)
    }

    fn find_rename(&self) -> Option<&LitStr> {
        self.attrs.iter().find_map(SegmentAttr::as_rename)
    }
}

impl Parse for Segment {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attrs: Punctuated<SegmentAttr, Token![,]> =
            input.parse_terminated(SegmentAttr::parse, Token![,])?;

        // Check for duplicates.
        let mut keys = HashSet::new();
        for attr in attrs.iter() {
            if !keys.insert(attr.keyword()) {
                return Err(syn::Error::new(
                    attr.span(),
                    format_args!("duplicate attribute `{}`", attr.keyword()),
                ));
            }
        }

        Ok(Self { attrs })
    }
}

impl ToTokens for Segment {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        if cfg!(feature = "discover") {
            let kind = self.find_kind();
            let rename = self.find_rename();

            let kind =
                QuoteOption::new(kind.map(|kind| quote! { ::topcoat::router::SegmentKind::#kind }));
            let rename = QuoteOption::new(
                rename.map(|rename| quote! { ::std::borrow::Cow::Borrowed(#rename) }),
            );
            quote! {
                ::topcoat::internal::inventory::submit! {
                    ::topcoat::router::Segment::new(
                        module_path!(),
                        #kind,
                        #rename,
                    )
                }
            }
            .to_tokens(tokens);
        }
    }
}

mod kw {
    use syn::custom_keyword;

    custom_keyword!(kind);
    custom_keyword!(rename);
}

#[expect(
    dead_code,
    reason = "parsed for syntax validation; not yet consumed by code generation"
)]
pub enum SegmentAttr {
    Kind {
        kind_kw: kw::kind,
        eq_token: Token![=],
        value: Ident,
    },
    Rename {
        rename_kw: kw::rename,
        eq_token: Token![=],
        value: LitStr,
    },
}

impl SegmentAttr {
    fn keyword(&self) -> &'static str {
        match self {
            Self::Kind { .. } => "kind",
            Self::Rename { .. } => "rename",
        }
    }

    fn span(&self) -> Span {
        match self {
            Self::Kind { kind_kw, .. } => kind_kw.span,
            Self::Rename { rename_kw, .. } => rename_kw.span,
        }
    }

    fn as_kind(&self) -> Option<&Ident> {
        match self {
            Self::Kind { value, .. } => Some(value),
            _ => None,
        }
    }

    fn as_rename(&self) -> Option<&LitStr> {
        match self {
            Self::Rename { value, .. } => Some(value),
            _ => None,
        }
    }
}

impl Parse for SegmentAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::kind) {
            Ok(Self::Kind {
                kind_kw: input.parse()?,
                eq_token: input.parse()?,
                value: input.parse()?,
            })
        } else if lookahead.peek(kw::rename) {
            Ok(Self::Rename {
                rename_kw: input.parse()?,
                eq_token: input.parse()?,
                value: input.parse()?,
            })
        } else {
            Err(lookahead.error())
        }
    }
}
