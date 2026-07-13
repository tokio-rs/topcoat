use proc_macro2::Span;
use syn::{spanned::Spanned, visit::Visit};

use crate::pretty::{INDENT, MARGIN};

pub struct MacroSnippet {
    name: String,
    source_text: String,
    span: Span,
    initial_space: isize,
    initial_indent: isize,
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
}

impl<'ast> Visit<'ast> for Visitor {
    fn visit_macro(&mut self, i: &'ast syn::Macro) {
        let name = &i.path.segments.last().expect("paths cannot be empty").ident;
        let span = i.delimiter.span().span();
        let source_text = span.source_text().unwrap();
        let initial_space = MARGIN - isize::try_from(span.start().column).unwrap();
        // Anchor the body to the indentation of the line the macro sits on, not
        // its depth in the AST: a macro can be indented by constructs that are
        // not blocks (a call argument, an array element, ...), and only the
        // source line reflects the column its body and closing delimiter align
        // to.
        let initial_indent = self.line_indent(span.start().line);

        self.snippets.push(MacroSnippet {
            name: name.to_string(),
            source_text,
            span,
            initial_space,
            initial_indent,
        });

        syn::visit::visit_macro(self, i);
    }
}
