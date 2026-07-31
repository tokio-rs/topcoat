#![cfg_attr(docsrs, feature(doc_cfg))]

use proc_macro::TokenStream;

#[doc = include_str!("../docs/shell_view.md")]
#[proc_macro]
pub fn shell_view(tokens: TokenStream) -> TokenStream {
    match syn::parse::<topcoat_shell_view_grammar::ShellView>(tokens)
        .and_then(|shell| shell.expand())
    {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}
