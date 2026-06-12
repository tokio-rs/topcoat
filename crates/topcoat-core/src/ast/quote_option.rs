use std::ops::{Deref, DerefMut};

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};

/// A newtype wrapper around `Option<T>` that implements [`ToTokens`] by
/// preserving the `Option` in the generated token stream.
///
/// The default `ToTokens` impl for `Option<T>` emits nothing for `None` and
/// emits the inner value directly for `Some`. `QuoteOption` instead quotes
/// `Some(value)` as `::core::option::Option::Some(value)` and `None` as
/// `::core::option::Option::None`, so the resulting code still contains an
/// `Option`.
///
/// Implements `Deref`/`DerefMut` to `Option<T>` for ergonomic access.
#[allow(unused)]
pub struct QuoteOption<T>(Option<T>);

impl<T> QuoteOption<T> {
    #[inline]
    #[allow(unused)]
    pub fn new(inner: Option<T>) -> Self {
        Self(inner)
    }
}

impl<T> Deref for QuoteOption<T> {
    type Target = Option<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for QuoteOption<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> ToTokens for QuoteOption<T>
where
    T: ToTokens,
{
    fn to_tokens(&self, tokens: &mut TokenStream) {
        if let Some(inner) = &self.0 {
            quote! { ::core::option::Option::Some(#inner) }
        } else {
            quote! { ::core::option::Option::None }
        }
        .to_tokens(tokens);
    }
}

impl<T> From<Option<T>> for QuoteOption<T> {
    fn from(value: Option<T>) -> Self {
        QuoteOption::new(value)
    }
}

impl<'a, T> From<&'a Option<T>> for QuoteOption<&'a T> {
    fn from(value: &'a Option<T>) -> Self {
        QuoteOption::new(value.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use super::*;

    #[test]
    fn some_quotes_as_option_some() {
        let opt = QuoteOption::new(Some(42i32));
        let tokens = quote! { #opt };
        let output = tokens.to_string();
        assert!(
            output.contains("Some"),
            "expected Option::Some, got: {output}"
        );
        assert!(
            output.contains("42"),
            "expected inner value 42, got: {output}"
        );
    }

    #[test]
    fn none_quotes_as_option_none() {
        let opt = QuoteOption::new(None::<i32>);
        let tokens = quote! { #opt };
        let output = tokens.to_string();
        assert!(
            output.contains("None"),
            "expected Option::None, got: {output}"
        );
    }

    #[test]
    fn deref_exposes_inner_option() {
        let opt = QuoteOption::new(Some(1));
        assert_eq!(*opt, Some(1));

        let opt = QuoteOption::new(None::<i32>);
        assert_eq!(*opt, None);
    }

    #[test]
    fn deref_mut_allows_mutation() {
        let mut opt = QuoteOption::new(None::<i32>);
        *opt = Some(5);
        assert_eq!(*opt, Some(5));
    }

    #[test]
    fn from_ref_option() {
        let val = Some(String::from("hello"));
        let opt: QuoteOption<&String> = QuoteOption::from(&val);
        assert_eq!(opt.as_ref().map(|s| s.as_str()), Some("hello"));

        let val: Option<String> = None;
        let opt: QuoteOption<&String> = QuoteOption::from(&val);
        assert_eq!(*opt, None);
    }
}
