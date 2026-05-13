mod view_writer_for_loop;
mod view_writer_if;
mod view_writer_match;

pub(crate) use view_writer_if::*;
pub(crate) use view_writer_match::*;

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};

/// Builds the `TokenStream` that a `view!` invocation expands to.
///
/// Adjacent literal markup is concatenated into `static_segment` and flushed as
/// a single write whenever a dynamic chunk (expression, control flow) appears.
/// `capacity` accumulates the lower bound of bytes the rendered view will need
/// so the runtime can pre-allocate the output buffer.
pub(crate) struct ViewWriter {
    pub(self) tokens: TokenStream,
    static_segment: String,
    capacity: usize,
    nested: bool,
}

impl ViewWriter {
    pub fn new() -> Self {
        Self {
            tokens: TokenStream::new(),
            static_segment: String::new(),
            capacity: 0,
            nested: false,
        }
    }

    pub fn new_nested() -> Self {
        Self {
            tokens: TokenStream::new(),
            static_segment: String::new(),
            capacity: 0,
            nested: true,
        }
    }

    fn flush(&mut self) {
        if !self.static_segment.is_empty() {
            let static_segment = &self.static_segment;
            quote! { ::topcoat::view::Fragment::fmt_unescaped(#static_segment, __cx, &mut __f); }
                .to_tokens(&mut self.tokens);
            self.capacity += self.static_segment.len();
            self.static_segment.clear();
        }
    }

    pub fn write_str_unescaped(&mut self, s: &str) {
        self.static_segment.push_str(s);
    }

    pub fn write_str(&mut self, s: &str) {
        crate::runtime::Formatter::new(&mut self.static_segment).write_str(s);
    }

    pub fn write_expr_unescaped(&mut self, expr: TokenStream) {
        self.flush();
        quote! { ::topcoat::view::Fragment::fmt_unescaped(&#expr, __cx, &mut __f); }
            .to_tokens(&mut self.tokens);
    }

    pub fn write_expr(&mut self, expr: TokenStream) {
        self.flush();
        quote! { ::topcoat::view::Fragment::fmt(&#expr, __cx, &mut __f); }
            .to_tokens(&mut self.tokens);
    }

    pub fn write_raw(&mut self, tokens: TokenStream) {
        tokens.to_tokens(&mut self.tokens);
    }
}

impl ToTokens for ViewWriter {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let format_block = {
            let static_segment = &self.static_segment;

            // Optimized path: The view has no dynamic content. We can construct it as a &'static str.
            if self.tokens.is_empty() {
                quote! { ::topcoat::view::View::new(::topcoat::view::ViewPart::StaticStr(#static_segment)) }
            } else {
                let buffer = &self.tokens;
                let capacity = self.capacity + static_segment.len();
                let final_segment = (!static_segment.is_empty()).then(|| {
                    quote! { ::topcoat::view::Fragment::fmt_unescaped(#static_segment, __cx, &mut __f); }
                });
                quote! {{
                    let mut __buf = ::std::string::String::with_capacity(#capacity);
                    let mut __f = ::topcoat::view::Formatter::new(&mut __buf);
                    #buffer
                    #final_segment
                    ::topcoat::view::View::new(__buf)
                }}
            }
        };

        if self.nested {
            format_block.to_tokens(tokens);
        } else {
            quote! { async { Ok(#format_block) }.await }.to_tokens(tokens);
        }
    }
}
