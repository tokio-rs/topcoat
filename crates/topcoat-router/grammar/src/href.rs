use proc_macro2::{Spacing, TokenStream, TokenTree};
use quote::{ToTokens, quote};
use syn::{
    Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};
use topcoat_core_grammar::common::{lenient, paths::topcoat_router};

/// The input to `href!`: a target followed by `param: value` arguments.
pub struct Href {
    pub target: lenient::Expr,
    pub comma_token: Option<Token![,]>,
    pub args: Punctuated<HrefArg, Token![,]>,
}

impl Href {
    /// Whether the target is shaped like a bare path: names joined by `::`,
    /// such as `post` or `posts::post`.
    ///
    /// A trailing `::` still counts. An editor completes a path that is still
    /// being typed by expanding it with a name filled in, which only works when
    /// both versions expand the same way.
    fn target_is_path(&self) -> bool {
        let mut tokens = self.target.tokens.clone().into_iter();
        let mut names = 0;
        let mut after_name = false;
        while let Some(token) = tokens.next() {
            match token {
                TokenTree::Ident(_) if !after_name => {
                    names += 1;
                    after_name = true;
                }
                // A `::` follows a name, or leads the path.
                TokenTree::Punct(first)
                    if first.as_char() == ':'
                        && first.spacing() == Spacing::Joint
                        && (after_name || names == 0) =>
                {
                    let Some(TokenTree::Punct(second)) = tokens.next() else {
                        return false;
                    };
                    if second.as_char() != ':' {
                        return false;
                    }
                    after_name = false;
                }
                _ => return false,
            }
        }
        names > 0
    }

    /// The target as an expression.
    ///
    /// A bare path names a handler's marker and is resolved as a type: inside
    /// the handler's own body, the handler function shadows the marker in the
    /// value namespace, so only the type namespace reaches it.
    fn target_expr(&self) -> TokenStream {
        let target = &self.target;
        if self.target_is_path() {
            quote! { <#target as ::core::default::Default>::default() }
        } else {
            target.to_token_stream()
        }
    }
}

impl Parse for Href {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            target: input.parse()?,
            comma_token: input.parse()?,
            args: Punctuated::parse_terminated(input)?,
        })
    }
}

impl ToTokens for Href {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let target = self.target_expr();
        let args = self.args.iter().map(|arg| {
            let key = &arg.param;
            let value = &arg.value;
            quote! {
                #key::__href(#value)
            }
        });
        quote! {
            #topcoat_router::Href::new(#target, (#(#args,)*))
        }
        .to_tokens(tokens);
    }
}

/// Prints the target followed by the arguments, breaking the arguments onto
/// their own lines when they do not fit. A trailing comma without arguments is
/// dropped.
#[cfg(feature = "pretty")]
impl topcoat_core_grammar::common::pretty::PrettyPrint for Href {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::common::pretty::Printer<'_>) {
        use topcoat_core_grammar::common::pretty::TextMode;

        self.target.pretty_print(printer);

        let Some(comma_token) = &self.comma_token else {
            return;
        };
        if self.args.is_empty() {
            printer.move_cursor(comma_token.span.end());
            return;
        }

        comma_token.pretty_print(printer);
        for (index, pair) in self.args.pairs().enumerate() {
            printer.scan_same_line_trivia();
            printer.scan_break();
            " ".pretty_print(printer);
            printer.scan_trivia(true, true);
            pair.value().pretty_print(printer);
            if pair.punct().is_some() {
                printer.scan_no_break_trivia();
            }
            match pair.punct() {
                Some(punct) if index < self.args.len() - 1 => punct.pretty_print(printer),
                _ => {
                    printer.scan_text(",".into(), TextMode::Break);
                    printer.advance_cursor(",");
                }
            }
        }
    }
}

/// A `param: value` argument of `href!`, filling the path parameter `param`
/// with `value`.
///
/// The parameter can be a path, so a parameter declared in another module can
/// be named without importing it, as in `posts::post_id: 1`. The colon and the
/// value can be missing, so an argument that is still being typed keeps the
/// macro parsing. A missing value is an empty [`lenient::Expr`], which expands
/// to a compile error.
pub struct HrefArg {
    pub param: lenient::Expr,
    pub colon_token: Option<Token![:]>,
    pub value: lenient::Expr,
}

impl Parse for HrefArg {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let param = input.parse()?;
        let colon_token: Option<Token![:]> = input.parse()?;
        let value = match colon_token {
            Some(_) => input.parse()?,
            None => lenient::Expr {
                tokens: TokenStream::new(),
                span: input.span(),
            },
        };
        Ok(Self {
            param,
            colon_token,
            value,
        })
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::common::pretty::PrettyPrint for HrefArg {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::common::pretty::Printer<'_>) {
        self.param.pretty_print(printer);
        self.colon_token.pretty_print(printer);
        if !self.value.is_empty() {
            " ".pretty_print(printer);
            self.value.pretty_print(printer);
        }
    }
}
