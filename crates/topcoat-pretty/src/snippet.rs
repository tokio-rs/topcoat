use proc_macro2::Span;
use syn::{spanned::Spanned, visit::Visit};

use crate::MARGIN;

pub struct MacroSnippet {
    name: String,
    source_text: String,
    span: Span,
    initial_space: isize,
    initial_indent: isize,
}

impl MacroSnippet {
    pub fn collect_from_file(file: &syn::File) -> Vec<Self> {
        let mut visitor = Visitor::default();
        visitor.visit_file(file);
        visitor.snippets
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn source_text(&self) -> &str {
        &self.source_text
    }

    pub fn span(&self) -> Span {
        self.span
    }

    pub fn initial_space(&self) -> isize {
        self.initial_space
    }

    pub fn initial_indent(&self) -> isize {
        self.initial_indent
    }
}

#[derive(Default)]
struct Visitor {
    indent: isize,
    snippets: Vec<MacroSnippet>,
}

impl<'ast> Visit<'ast> for Visitor {
    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        self.indent += 1;
        syn::visit::visit_item_mod(self, node);
        self.indent -= 1;
    }

    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        self.indent += 1;
        syn::visit::visit_item_impl(self, node);
        self.indent -= 1;
    }

    fn visit_item_trait(&mut self, node: &'ast syn::ItemTrait) {
        self.indent += 1;
        syn::visit::visit_item_trait(self, node);
        self.indent -= 1;
    }

    fn visit_block(&mut self, node: &'ast syn::Block) {
        self.indent += 1;
        syn::visit::visit_block(self, node);
        self.indent -= 1;
    }

    fn visit_macro(&mut self, i: &'ast syn::Macro) {
        let name = &i.path.segments.last().expect("paths cannot be empty").ident;
        let span = i.delimiter.span().span();
        let source_text = span.source_text().unwrap();
        let initial_space = MARGIN - isize::try_from(span.start().column).unwrap();
        let initial_indent = self.indent;

        self.snippets.push(MacroSnippet {
            name: name.to_string(),
            source_text,
            span,
            initial_space,
            initial_indent,
        });
    }
}
