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
    pub(self) exprs: Vec<TokenStream>,
    static_segment: String,
    capacity: usize,
    nested: bool,
}

impl ViewWriter {
    pub fn new() -> Self {
        Self {
            exprs: Vec::new(),
            static_segment: String::new(),
            capacity: 0,
            nested: false,
        }
    }

    pub fn new_nested() -> Self {
        Self {
            exprs: Vec::new(),
            static_segment: String::new(),
            capacity: 0,
            nested: true,
        }
    }

    fn flush(&mut self) {
        if !self.static_segment.is_empty() {
            let static_segment = &self.static_segment;
            self.exprs
                .push(quote! { ::topcoat::view::ViewPart::StaticStr(#static_segment) });
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

    pub fn write_expr(&mut self, expr: TokenStream) {
        self.flush();
        self.exprs.push(expr)
    }
}

impl ToTokens for ViewWriter {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let format_expr = {
            let static_segment = &self.static_segment;

            if self.exprs.is_empty() {
                // Optimized path: The view has no dynamic content. We can construct it as a &'static str.
                quote! { ::topcoat::view::View::new(#static_segment) }
            } else if self.exprs.len() == 1 && static_segment.is_empty() {
                // Optimized path: The view can be constructed from a single expression.
                let entry = self.exprs.first().unwrap();
                quote! { ::topcoat::view::View::new(#entry) }
            } else {
                let entries = &self.exprs;
                let final_segment = (!static_segment.is_empty()).then(|| {
                    quote! { ::topcoat::view::ViewPart::StaticStr(#static_segment) }
                });
                quote! {{
                    ::topcoat::view::View::new(Box::new([
                        #(::topcoat::view::IntoViewPart::into_view_part(#entries),)*
                        #final_segment
                    ]))
                }}
            }
        };

        if self.nested {
            format_expr.to_tokens(tokens);
        } else {
            quote! { async { Ok(#format_expr) }.await }.to_tokens(tokens);
        }
    }
}
