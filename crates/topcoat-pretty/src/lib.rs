mod delim;
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
pub use r#macro::*;
pub use pretty_print::*;
pub use printer::*;
pub use registry::*;
pub use ring_buffer::*;
pub use snippet::*;
pub use span::*;
pub use token::*;
pub use trivia::*;

struct Replace {
    start: usize,
    end: usize,
    replacement: String,
}

/// Pretty-prints the Topcoat macro invocations in `input`.
///
/// Returns the source with each macro body replaced by its formatted form, or
/// the collection of errors encountered while parsing or formatting them.
///
/// # Errors
///
/// Returns `Err` with the accumulated `syn::Error`s if parsing `input` or any
/// macro body fails, or if a registered pretty-printer returns an error.
pub fn pretty_print_str(registry: &Registry, input: &str) -> Result<String, Vec<syn::Error>> {
    let mut output = String::new();

    let file = syn::parse_file(input).map_err(|error| vec![error])?;
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
            Err(error) => errors.push(error),
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
