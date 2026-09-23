use syn::parse::{Parse, ParseStream};

/// Parses a value only when the next tokens match its syntax.
///
/// Implement [`peek`](Self::peek) to recognize the start of a value.
///
/// # Example
///
/// ```rust
/// use syn::{
///     Token,
///     parse::{Parse, ParseStream},
/// };
/// use topcoat_core_grammar::ParseOption;
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
    fn peek(input: ParseStream) -> bool;

    /// Optionally parse this type from the input stream.
    ///
    /// If `peek` returns `true`, this method will attempt to parse the type.
    /// Otherwise, it returns `Ok(None)`.
    ///
    /// # Errors
    ///
    /// Returns an error from `Parse::parse` if `peek` returned `true` but the
    /// input could not be parsed as `Self`.
    fn parse_option(input: ParseStream) -> syn::Result<Option<Self>> {
        Self::peek(input).then(|| input.parse()).transpose()
    }
}
