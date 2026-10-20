#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "fluent")]
use std::path::PathBuf;

#[cfg(feature = "fluent")]
use proc_macro::{Span, TokenStream};

#[cfg(feature = "fluent")]
#[doc = include_str!("../docs/fluent/messages.md")]
#[proc_macro]
pub fn messages(tokens: TokenStream) -> TokenStream {
    use topcoat_l10n_grammar::fluent::{CallSite, Messages};

    let parsed = syn::parse_macro_input!(tokens as Messages);
    let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_default();
    let source_file = Span::call_site().local_file();
    match CallSite::new(&manifest_dir, source_file.as_deref()) {
        Ok(call_site) => parsed.expand(&call_site).into(),
        Err(err) => syn::Error::new(Span::call_site().into(), err)
            .to_compile_error()
            .into(),
    }
}
