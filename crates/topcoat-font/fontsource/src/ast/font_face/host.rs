use syn::{
    Token,
    parse::{Parse, ParseStream},
};

use topcoat_core::ast::ParseOption;

mod kw {
    use syn::custom_keyword;

    custom_keyword!(host);
    custom_keyword!(asset);
}

/// A `host: asset` argument, opting the face into a self-hosted, bundled copy
/// instead of the jsDelivr CDN. Its presence is the whole signal; there is one
/// possible value.
pub struct Host {
    pub key: HostKey,
    pub colon_token: Token![:],
    pub value: HostValue,
}

impl Parse for Host {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            key: input.parse()?,
            colon_token: input.parse()?,
            value: input.parse()?,
        })
    }
}

impl ParseOption for Host {
    fn peek(input: ParseStream) -> bool {
        HostKey::peek(input)
    }
}

pub struct HostKey {
    pub host_kw: kw::host,
}

impl Parse for HostKey {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            host_kw: input.parse()?,
        })
    }
}

impl ParseOption for HostKey {
    fn peek(input: ParseStream) -> bool {
        input.peek(kw::host)
    }
}

/// The sole host value, the `asset` keyword.
pub struct HostValue {
    pub asset_kw: kw::asset,
}

impl Parse for HostValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            asset_kw: input.parse()?,
        })
    }
}
