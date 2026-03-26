use std::borrow::Cow;

use proc_macro::{Delimiter, TokenStream, TokenTree};
use quote::quote;
use topcoat_view::token::{Span, Token, TokenKind};

#[proc_macro]
pub fn view(tokens: TokenStream) -> TokenStream {
    struct Convert<'a> {
        result: Vec<Token<'a>>,
    }

    impl<'a> Convert<'a> {
        fn convert(&mut self, tokens: TokenStream) -> Result<(), TokenStream> {
            for token in tokens.into_iter() {
                match token {
                    TokenTree::Group(group) => {
                        use TokenKind as TK;
                        let (open, open_text, close, close_text) = match group.delimiter() {
                            Delimiter::Parenthesis => (TK::LParen, "(", TK::RParen, ")"),
                            Delimiter::Brace => (TK::LBrace, "{", TK::RBrace, "}"),
                            Delimiter::Bracket => (TK::LBracket, "[", TK::RBracket, "]"),
                            Delimiter::None => (TK::Whitespace, "", TK::Whitespace, ""),
                        };
                        self.emit(open, open_text, group.span_open());
                        self.convert(group.stream())?;
                        self.emit(close, close_text, group.span_close());
                    }
                    TokenTree::Ident(ident) => {
                        self.emit(TokenKind::Ident, ident.to_string(), ident.span())
                    }
                    TokenTree::Literal(literal) => {
                        self.emit(TokenKind::Ident, literal.to_string(), literal.span())
                    }
                    TokenTree::Punct(punct) => {
                        let kind = match punct.as_char() {
                            '=' => TokenKind::Eq,
                            _ => {
                                return Err(compile_error(
                                    &format!("unexpected character `{}`", punct),
                                    punct.span(),
                                ));
                            }
                        };
                        self.emit(kind, punct.to_string(), punct.span());
                    }
                }
            }
            Ok(())
        }

        fn emit(&mut self, kind: TokenKind, text: impl Into<Cow<'a, str>>, span: proc_macro::Span) {
            self.result.push(Token::new(kind, text, Span::new(0, 0)))
        }
    }

    let mut converter = Convert { result: Vec::new() };
    match converter.convert(tokens) {
        Ok(()) => {}
        Err(err) => return err,
    };

    let debug = format!("{:?}", converter.result);

    quote! { println!("{}", #debug); }.into()
}

fn compile_error(msg: &str, span: proc_macro::Span) -> TokenStream {
    let ts: TokenStream = format!("compile_error!({:?});", msg).parse().unwrap();
    ts.into_iter()
        .map(|mut t| {
            t.set_span(span);
            t
        })
        .collect()
}
