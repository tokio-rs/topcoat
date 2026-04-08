mod view_writer_for_loop;
mod view_writer_if;
mod view_writer_match;

use syn::ExprLet;
pub(crate) use view_writer_if::*;
pub(crate) use view_writer_match::*;

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};

#[derive(Default)]
pub(crate) struct ViewWriter {
    pub(self) tokens: TokenStream,
    static_segment: String,
    capacity: usize,
}

impl ViewWriter {
    pub fn new() -> Self {
        Self::default()
    }

    fn flush(&mut self) {
        if !self.static_segment.is_empty() {
            let static_segment = &self.static_segment;
            quote! { writer.push_fragment(#static_segment); }.to_tokens(&mut self.tokens);
            self.capacity += self.static_segment.len();
            self.static_segment.clear();
        }
    }

    pub fn push(&mut self, ch: char) {
        self.static_segment.push(ch);
    }

    pub fn push_str(&mut self, string: &str) {
        self.static_segment.push_str(string);
    }

    pub fn push_escaped(&mut self, string: &str) {
        for c in string.chars() {
            match c {
                '&' => self.push_str("&amp;"),
                '<' => self.push_str("&lt;"),
                '>' => self.push_str("&gt;"),
                '"' => self.push_str("&quot;"),
                '\'' => self.push_str("&#x27;"),
                _ => self.push(c),
            }
        }
    }

    pub fn push_expr(&mut self, expr: TokenStream) {
        self.flush();
        quote! { writer.push_fragment(#expr); }.to_tokens(&mut self.tokens);
    }

    pub fn push_expr_let(&mut self, expr_let: &ExprLet) {
        quote! { #expr_let; }.to_tokens(&mut self.tokens);
    }
}

impl ToTokens for ViewWriter {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let static_segment = &self.static_segment;

        // Optimized path: The view has no dynamic content. We can construct it as a &'static str.
        if self.tokens.is_empty() {
            quote! { ::topcoat::view::View::new(#static_segment) }.to_tokens(tokens);
            return;
        }

        let buffer = &self.tokens;
        let capacity = self.capacity + static_segment.len();
        let final_segment = (!static_segment.is_empty()).then(|| {
            quote! { writer.push_fragment(#static_segment); }
        });
        quote! {{
            let mut writer = ::topcoat::view::ViewWriter::with_capacity(#capacity);
            #buffer
            #final_segment
            writer.finish()
        }}
        .to_tokens(tokens);
    }
}
