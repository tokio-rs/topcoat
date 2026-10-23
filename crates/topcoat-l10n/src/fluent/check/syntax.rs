use std::path::Path;

use fluent_syntax::{
    ast::{Expression, InlineExpression, Pattern, PatternElement, Resource},
    parser,
};

use super::Location;

/// Fluent syntax parsed from a source file.
pub(super) struct ParsedFile<'s> {
    path: &'s Path,
    text: &'s str,
    pub(super) resource: Resource<&'s str>,
}

impl<'s> ParsedFile<'s> {
    /// Parses the contents of `path`.
    ///
    /// Keeps valid entries if parsing finds errors. The `messages!` macro already
    /// rejects syntax errors during compilation.
    pub(super) fn parse(path: &'s Path, text: &'s str) -> Self {
        let resource = parser::parse(text).unwrap_or_else(|(resource, _)| resource);
        Self {
            path,
            text,
            resource,
        }
    }

    /// Finds the source location of a slice of the file, such as an identifier.
    pub(super) fn location(&self, slice: &str) -> Location {
        let offset = slice
            .as_ptr()
            .addr()
            .checked_sub(self.text.as_ptr().addr())
            .filter(|offset| *offset <= self.text.len())
            .unwrap_or(0);
        let before = &self.text[..offset];
        let line_start = before.rfind('\n').map_or(0, |index| index + 1);
        let line = before.matches('\n').count() + 1;
        let column = before[line_start..].chars().count() + 1;
        Location {
            file: self.path.to_owned(),
            line: u32::try_from(line).unwrap_or(u32::MAX),
            column: u32::try_from(column).ok(),
        }
    }
}

/// Visits all expressions in a pattern, including expressions inside selectors,
/// variants and call arguments.
pub(super) fn visit_expressions<'s>(
    pattern: &'s Pattern<&'s str>,
    visit: &mut dyn FnMut(&'s InlineExpression<&'s str>),
) {
    for element in &pattern.elements {
        if let PatternElement::Placeable { expression } = element {
            visit_expression(expression, visit);
        }
    }
}

fn visit_expression<'s>(
    expression: &'s Expression<&'s str>,
    visit: &mut dyn FnMut(&'s InlineExpression<&'s str>),
) {
    match expression {
        Expression::Select { selector, variants } => {
            visit_inline(selector, visit);
            for variant in variants {
                visit_expressions(&variant.value, visit);
            }
        }
        Expression::Inline(inline) => visit_inline(inline, visit),
    }
}

fn visit_inline<'s>(
    inline: &'s InlineExpression<&'s str>,
    visit: &mut dyn FnMut(&'s InlineExpression<&'s str>),
) {
    visit(inline);
    match inline {
        InlineExpression::FunctionReference { arguments, .. }
        | InlineExpression::TermReference {
            arguments: Some(arguments),
            ..
        } => {
            for argument in &arguments.positional {
                visit_inline(argument, visit);
            }
            for argument in &arguments.named {
                visit_inline(&argument.value, visit);
            }
        }
        InlineExpression::Placeable { expression } => visit_expression(expression, visit),
        _ => {}
    }
}
