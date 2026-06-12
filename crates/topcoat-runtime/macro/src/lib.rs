use proc_macro::TokenStream;

#[proc_macro]
pub fn expr(tokens: TokenStream) -> TokenStream {
    let parsed = syn::parse_macro_input!(tokens as topcoat_runtime::ast::expr::Expr);
    match parsed.expr_to_tokens() {
        Ok(ts) => ts.into(),
        Err(err) => err.to_compile_error().into(),
    }
}
