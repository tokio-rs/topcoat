//! Assertions for testing macro grammars.

use quote::ToTokens;

/// How many times [`assert_deterministic`] expands its input.
///
/// Each expansion gets fresh hash seeds, so output that depends on hash
/// iteration order is likely to differ between at least two of them.
const EXPANSIONS: usize = 4;

/// Asserts that a macro expands to the same tokens every time.
///
/// `expand` must parse the macro input and expand it from scratch on each
/// call, so that state shared between expansions, random values, and hash
/// iteration order all show up as differences in the output. The compiler
/// can only reuse incremental results for an expansion that is unchanged
/// between builds.
///
/// ```
/// use topcoat_core_grammar::testing::assert_deterministic;
///
/// assert_deterministic(|| syn::parse_str::<syn::Expr>("1 + 2"));
/// ```
///
/// # Panics
///
/// Panics if `expand` returns an error, or if two expansions differ.
pub fn assert_deterministic<T: ToTokens>(expand: impl Fn() -> syn::Result<T>) {
    let run = || match expand() {
        Ok(value) => value.into_token_stream().to_string(),
        Err(error) => panic!("failed to parse the macro input: {error}"),
    };

    let first = run();
    for _ in 1..EXPANSIONS {
        let other = run();
        if first != other {
            let at = first
                .bytes()
                .zip(other.bytes())
                .take_while(|(a, b)| a == b)
                .count();
            panic!(
                "macro expanded to different tokens on repeated runs\n\nfirst:  ...{}\nlater:  ...{}",
                excerpt(&first, at),
                excerpt(&other, at),
            );
        }
    }
}

/// Returns the text of `expansion` around byte offset `at`.
fn excerpt(expansion: &str, at: usize) -> &str {
    let start = expansion.floor_char_boundary(at.saturating_sub(80));
    let end = expansion.ceil_char_boundary(at + 80);
    &expansion[start..end]
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use proc_macro2::{Span, TokenStream};
    use quote::quote;
    use syn::Ident;

    use super::*;

    #[test]
    fn accepts_identical_expansions() {
        assert_deterministic(|| Ok(quote! { fn generated() {} }));
    }

    #[test]
    #[should_panic(expected = "different tokens")]
    fn rejects_expansions_that_depend_on_shared_state() {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        assert_deterministic(|| {
            let n = COUNTER.fetch_add(1, Ordering::Relaxed);
            let ident = Ident::new(&format!("__generated_{n}"), Span::call_site());
            Ok(quote! { let #ident = 1; })
        });
    }

    #[test]
    #[should_panic(expected = "failed to parse")]
    fn rejects_inputs_that_fail_to_parse() {
        assert_deterministic(|| syn::parse_str::<TokenStream>("("));
    }
}
