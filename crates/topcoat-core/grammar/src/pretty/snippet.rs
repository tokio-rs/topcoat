use proc_macro2::{Span, TokenStream};
use syn::{
    Expr, MacroDelimiter, Token,
    parse::{ParseStream, Parser},
    punctuated::Punctuated,
    spanned::Spanned,
    visit::Visit,
};

use crate::pretty::{INDENT, MARGIN};

/// The kind of invocation a [`MacroSnippet`] holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacroKind {
    /// A function-like macro call, such as `view! { ... }`.
    FunctionLike,
    /// The arguments of an attribute, such as the `("/")` in `#[page("/")]`.
    Attribute,
}

pub struct MacroSnippet {
    kind: MacroKind,
    name: String,
    source_text: String,
    span: Span,
    initial_space: isize,
    initial_indent: isize,
    laid_out_by_rustfmt: bool,
}

impl MacroSnippet {
    #[must_use]
    pub fn collect_from_file(file: &syn::File) -> Vec<Self> {
        // `file.span()` covers the whole file, so its source text is the
        // original source and its start line is the line that text begins on.
        // Both are needed to recover each macro's indentation from the actual
        // layout rather than its depth in the AST.
        let span = file.span();
        let mut visitor = Visitor {
            source: span.source_text().unwrap_or_default(),
            first_line: span.start().line,
            snippets: Vec::new(),
        };
        visitor.visit_file(file);
        visitor.snippets
    }

    /// Builds a snippet for a single invocation encountered while printing the
    /// Rust code around it, laid out at the printer's current indentation.
    /// Returns `None` when the invocation has no source text to reprint from.
    pub(crate) fn from_macro(mac: &syn::Macro, initial_indent: isize) -> Option<Self> {
        Self::nested(MacroKind::FunctionLike, &mac.path, &mac.delimiter, initial_indent)
    }

    /// Builds a snippet for the arguments of an attribute encountered while
    /// printing the Rust code around it, laid out at the printer's current
    /// indentation. Returns `None` when the arguments have no source text to
    /// reprint from.
    pub(crate) fn from_attribute(list: &syn::MetaList, initial_indent: isize) -> Option<Self> {
        Self::nested(
            MacroKind::Attribute,
            &list.path,
            &list.delimiter,
            initial_indent,
        )
    }

    fn nested(
        kind: MacroKind,
        path: &syn::Path,
        delimiter: &MacroDelimiter,
        initial_indent: isize,
    ) -> Option<Self> {
        let name = &path.segments.last()?.ident;
        let span = delimiter.span().span();
        let source_text = span.source_text()?;
        Some(Self {
            kind,
            name: name.to_string(),
            source_text,
            span,
            // The output column is not known while tokens are being scanned;
            // the source column is the best estimate and makes reformatting
            // already-formatted code stable. One column is held back for the
            // closing delimiter or comma that usually follows the invocation.
            initial_space: MARGIN - isize::try_from(span.start().column).unwrap_or(0) - 1,
            initial_indent,
            laid_out_by_rustfmt: false,
        })
    }

    #[must_use]
    pub fn kind(&self) -> MacroKind {
        self.kind
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn source_text(&self) -> &str {
        &self.source_text
    }

    #[must_use]
    pub fn span(&self) -> Span {
        self.span
    }

    #[must_use]
    pub fn initial_space(&self) -> isize {
        self.initial_space
    }

    #[must_use]
    pub fn initial_indent(&self) -> isize {
        self.initial_indent
    }

    /// Whether `rustfmt` lays this invocation out itself.
    ///
    /// `rustfmt` reflows a macro call delimited by `()` or `[]` whose body is a
    /// comma-separated list of Rust expressions, treating it like a function
    /// call. It also reflows attribute arguments that form a list of
    /// literals, paths, `name(...)` lists and `name = literal` pairs. Laying
    /// such a body out here as well leaves the two formatters undoing each
    /// other's work. Everything `rustfmt` copies through verbatim belongs to
    /// this formatter.
    #[must_use]
    pub fn laid_out_by_rustfmt(&self) -> bool {
        self.laid_out_by_rustfmt
    }
}

struct Visitor {
    source: String,
    first_line: usize,
    snippets: Vec<MacroSnippet>,
}

impl Visitor {
    /// The indentation level of the given 1-based source line, measured as its
    /// count of leading whitespace characters divided by [`INDENT`].
    fn line_indent(&self, line: usize) -> isize {
        let index = line.saturating_sub(self.first_line);
        let Some(text) = self.source.lines().nth(index) else {
            return 0;
        };
        let leading = text.len() - text.trim_start().len();
        isize::try_from(leading).unwrap_or(0) / INDENT
    }

    fn push(
        &mut self,
        kind: MacroKind,
        path: &syn::Path,
        delimiter: &MacroDelimiter,
        laid_out_by_rustfmt: bool,
    ) {
        let name = &path.segments.last().expect("paths cannot be empty").ident;
        let span = delimiter.span().span();
        let source_text = span.source_text().unwrap();
        let initial_space = MARGIN - isize::try_from(span.start().column).unwrap();
        // Anchor the body to the indentation of the line the macro sits on, not
        // its depth in the AST: a macro can be indented by constructs that are
        // not blocks (a call argument, an array element, ...), and only the
        // source line reflects the column its body and closing delimiter align
        // to.
        let initial_indent = self.line_indent(span.start().line);

        self.snippets.push(MacroSnippet {
            kind,
            name: name.to_string(),
            source_text,
            span,
            initial_space,
            initial_indent,
            laid_out_by_rustfmt,
        });
    }
}

impl<'ast> Visit<'ast> for Visitor {
    fn visit_macro(&mut self, i: &'ast syn::Macro) {
        let laid_out_by_rustfmt = !matches!(i.delimiter, MacroDelimiter::Brace(_))
            && Punctuated::<Expr, Token![,]>::parse_terminated
                .parse2(i.tokens.clone())
                .is_ok();
        self.push(MacroKind::FunctionLike, &i.path, &i.delimiter, laid_out_by_rustfmt);

        syn::visit::visit_macro(self, i);
    }

    fn visit_attribute(&mut self, i: &'ast syn::Attribute) {
        if let syn::Meta::List(list) = &i.meta {
            let laid_out_by_rustfmt = matches!(list.delimiter, MacroDelimiter::Paren(_))
                && is_meta_list(list.tokens.clone());
            self.push(
                MacroKind::Attribute,
                &list.path,
                &list.delimiter,
                laid_out_by_rustfmt,
            );
        }

        syn::visit::visit_attribute(self, i);
    }
}

/// Whether `tokens` are attribute arguments that `rustfmt` reflows: a comma
/// list of literals, paths, `name(...)` lists and `name = literal` pairs.
fn is_meta_list(tokens: TokenStream) -> bool {
    fn items(input: ParseStream) -> syn::Result<()> {
        while !input.is_empty() {
            item(input)?;
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(())
    }

    fn item(input: ParseStream) -> syn::Result<()> {
        // `syn` reads `-1` as one literal, but `rustfmt` does not.
        if input.peek(Token![-]) {
            return Err(input.error("expected a literal or a path"));
        }
        if input.peek(syn::Lit) {
            input.parse::<syn::Lit>()?;
            return Ok(());
        }
        input.call(syn::Path::parse_mod_style)?;
        if input.peek(syn::token::Paren) {
            let content;
            syn::parenthesized!(content in input);
            items(&content)
        } else if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            input.parse::<syn::Lit>()?;
            Ok(())
        } else {
            Ok(())
        }
    }

    items.parse2(tokens).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attributes(source: &str) -> Vec<MacroSnippet> {
        let file = syn::parse_file(source).unwrap();
        MacroSnippet::collect_from_file(&file)
            .into_iter()
            .filter(|snippet| snippet.kind() == MacroKind::Attribute)
            .collect()
    }

    fn laid_out_by_rustfmt(attribute: &str) -> bool {
        let snippets = attributes(&format!("{attribute}\nfn f() {{}}"));
        assert_eq!(snippets.len(), 1, "{attribute}");
        snippets[0].laid_out_by_rustfmt()
    }

    #[test]
    fn collects_attribute_arguments() {
        let snippets = attributes("#[page(\"/\")]\nfn f() {}");
        assert_eq!(snippets.len(), 1);
        assert_eq!(snippets[0].name(), "page");
        assert_eq!(snippets[0].source_text(), "(\"/\")");
    }

    #[test]
    fn skips_attributes_without_arguments() {
        assert!(attributes("#[page]\n#[doc = \"x\"]\nfn f() {}").is_empty());
    }

    #[test]
    fn names_a_path_attribute_by_its_last_segment() {
        let snippets = attributes("#[topcoat::router::page(\"/\")]\nfn f() {}");
        assert_eq!(snippets[0].name(), "page");
    }

    #[test]
    fn meta_lists_belong_to_rustfmt() {
        assert!(laid_out_by_rustfmt("#[page(\"/\")]"));
        assert!(laid_out_by_rustfmt("#[route(GET)]"));
        assert!(laid_out_by_rustfmt("#[memoize(as_ref)]"));
        assert!(laid_out_by_rustfmt(
            "#[page(\"/\", with(Title(\"About\")), with(Index))]"
        ));
        // A call without arguments reads as a path followed by an empty list.
        assert!(laid_out_by_rustfmt("#[page(with(Head::new()))]"));
        assert!(laid_out_by_rustfmt("#[serde(rename = \"id\", default)]"));
    }

    #[test]
    fn other_arguments_belong_to_this_formatter() {
        // A method followed by a path without a comma.
        assert!(!laid_out_by_rustfmt("#[route(GET \"/\")]"));
        // A bracketed method list.
        assert!(!laid_out_by_rustfmt("#[route([GET, POST])]"));
        // A path as the value of `=`.
        assert!(!laid_out_by_rustfmt("#[query_params(error = bad_request)]"));
        // Expressions in a `with(...)` entry that are not paths and lists.
        assert!(!laid_out_by_rustfmt(
            "#[page(with(Head::new().title(\"About\")))]"
        ));
        assert!(!laid_out_by_rustfmt("#[page(with(Title(format!(\"{x}\"))))]"));
        assert!(!laid_out_by_rustfmt("#[limit(-1)]"));
        // A delimiter other than parentheses.
        assert!(!laid_out_by_rustfmt("#[page[\"/\"]]"));
    }
}
