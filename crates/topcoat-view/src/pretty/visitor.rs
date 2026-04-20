use syn::{spanned::Spanned, visit::Visit};

use super::{MARGIN, Macro, pretty_print_str};

pub fn pretty_print_rust_str(input: &str) -> Result<String, Vec<syn::Error>> {
    let mut output = String::new();

    let file = syn::parse_file(input).map_err(|error| vec![error])?;
    let mut visitor = Visitor::default();
    visitor.visit_file(&file);

    if !visitor.errors.is_empty() {
        return Err(visitor.errors);
    }

    let mut current_index = 0;
    for replacement in visitor.replacements {
        output.push_str(&input[current_index..replacement.start]);
        output.push_str(&replacement.replacement);
        current_index = replacement.end;
    }

    output.push_str(&input[current_index..]);

    Ok(output)
}

pub(super) struct Replace {
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) replacement: String,
}

#[derive(Default)]
pub(super) struct Visitor {
    pub(super) indent: isize,
    pub(super) replacements: Vec<Replace>,
    pub(super) errors: Vec<syn::Error>,
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

        let result = match name.to_string().as_ref() {
            "view" => Some(pretty_print_str::<Macro<crate::ast::View>>(
                &source_text,
                initial_space,
                initial_indent,
            )),
            _ => None,
        };

        match result {
            Some(Ok(replacement)) => self.replacements.push(Replace {
                start: span.byte_range().start,
                end: span.byte_range().end,
                replacement,
            }),
            Some(Err(error)) => {
                self.errors.push(error);
            }
            None => {}
        }
    }
}
