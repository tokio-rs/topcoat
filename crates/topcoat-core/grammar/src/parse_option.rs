use syn::parse::{Parse, ParseStream};

/// A type that may or may not be present at the current position of a
/// `ParseStream`.
///
/// Implement [`peek`](Self::peek) to tell whether the type starts at the
/// current position. [`parse_option`](Self::parse_option) then parses it only
/// when it is there.
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
    /// Returns whether the input starts with this type, without consuming
    /// any tokens.
    fn peek(input: ParseStream) -> bool;

    /// Parses this type if [`peek`](Self::peek) returns `true`, and returns
    /// `Ok(None)` otherwise.
    ///
    /// # Errors
    ///
    /// Returns an error from `Parse::parse` if `peek` returned `true` but the
    /// input could not be parsed as `Self`.
    fn parse_option(input: ParseStream) -> syn::Result<Option<Self>> {
        Self::peek(input).then(|| input.parse()).transpose()
    }
}
