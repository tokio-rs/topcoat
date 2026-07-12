mod delim;
mod error;
mod r#macro;
mod pretty_print;
mod printer;
mod registry;
mod ring_buffer;
mod rust;
mod snippet;
mod span;
mod text;
mod token;
mod trivia;

pub use delim::*;
pub use error::*;
pub use r#macro::*;
pub use pretty_print::*;
pub use printer::*;
pub use registry::*;
pub use ring_buffer::*;
pub use snippet::*;
pub use span::*;
pub use token::*;
pub use trivia::*;

use proc_macro2::LineColumn;

struct Replace {
    start: usize,
    end: usize,
    replacement: String,
}

/// Pretty-prints the Topcoat macro invocations in `input`.
///
/// Returns the source with each macro body replaced by its formatted form, or
/// the collection of errors encountered while parsing or formatting them. Each
/// error's location is reported in the coordinates of `input`.
///
/// # Errors
///
/// Returns `Err` with the accumulated [`FormatError`]s if parsing `input` or any
/// macro body fails, or if a registered pretty-printer returns an error.
pub fn pretty_print_str(registry: &Registry, input: &str) -> Result<String, Vec<FormatError>> {
    let mut output = String::new();

    // The whole file is parsed as a unit, so this error's span already refers to
    // positions in `input`; its body starts at the very first line and column.
    let file = syn::parse_file(input)
        .map_err(|error| vec![FormatError::new(&error, LineColumn { line: 1, column: 0 })])?;
    let snippets = MacroSnippet::collect_from_file(&file);
    let mut errors = Vec::new();
    let mut replacements = Vec::new();

    for snippet in snippets {
        let Some(replacement) = registry.pretty_print_macro(&snippet) else {
            continue;
        };
        match replacement {
            Ok(replacement) => replacements.push(Replace {
                start: snippet.span().byte_range().start,
                end: snippet.span().byte_range().end,
                replacement,
            }),
            // A body is parsed in isolation, so its error span is relative to
            // the body; anchor it to where the body begins in the file.
            Err(error) => errors.push(FormatError::new(&error, snippet.span().start())),
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    let mut current_index = 0;
    for replacement in replacements {
        output.push_str(&input[current_index..replacement.start]);
        output.push_str(&replacement.replacement);
        current_index = replacement.end;
    }

    output.push_str(&input[current_index..]);

    Ok(output)
}
