use syn::parse::{Parse, ParseStream};

/// A trait for types that can be optionally parsed from a `ParseStream`.
///
/// This trait provides a default implementation of `parse_option` that
/// uses the `peek` method to check if the type should be parsed.
///
/// # Example
///
/// ```rust
/// use syn::{
///     Token,
///     parse::{Parse, ParseStream},
/// };
/// use topcoat_core::ast::ParseOption;
///
/// struct MyStruct {
///     token: Token![as],
///     // ...
/// }
///
/// impl ParseOption for MyStruct {
///     fn peek(input: ParseStream) -> bool {
///         input.peek(Token![as])
///     }
/// }
///
/// impl Parse for MyStruct {
///     fn parse(input: ParseStream) -> syn::Result<Self> {
///         // ... parsing logic
/// #       Ok(MyStruct { token: input.parse()? })
///     }
/// }
/// ```
pub trait ParseOption: Parse + Sized {
    /// Check if the input stream has the expected token(s) for this type.
    ///
    /// This method should peek at the input without consuming any tokens.
    ///
    /// Note: `ParseStream` is actually `&ParseBuffer`, so implementations
    /// may use either type signature.
    fn peek(input: ParseStream) -> bool;

    /// Optionally parse this type from the input stream.
    ///
    /// If `peek` returns `true`, this method will attempt to parse the type.
    /// Otherwise, it returns `Ok(None)`.
    ///
    /// This method has a default implementation that uses `peek` and `Parse::parse`.
    ///
    /// # Errors
    ///
    /// Returns an error from `Parse::parse` if `peek` returned `true` but the
    /// input could not be parsed as `Self`.
    fn parse_option(input: ParseStream) -> syn::Result<Option<Self>> {
        Self::peek(input).then(|| input.parse()).transpose()
    }
}
