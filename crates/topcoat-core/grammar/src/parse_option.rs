use syn::parse::{Parse, ParseStream};

/// A trait for types that can be optionally parsed from a `ParseStream`.
///
/// Implement [`peek`](Self::peek) to recognize the start of the value. The
/// default [`parse_option`](Self::parse_option) parses it only when present.
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
    /// Returns whether the next tokens can start this type.
    ///
    /// This method should peek at the input without consuming any tokens.
    fn peek(input: ParseStream) -> bool;

    /// Parses this type when its opening tokens are present.
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
