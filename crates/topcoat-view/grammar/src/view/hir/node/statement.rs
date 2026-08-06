use proc_macro2::TokenStream;

/// A verbatim Rust statement.
pub(crate) struct Statement {
    pub tokens: TokenStream,
}
