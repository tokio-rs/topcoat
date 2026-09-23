use std::fmt::Write;

use crate::Font;

/// A function that writes the URL of a [`Font`]'s stylesheet.
pub type ResolveFontRouteFn = dyn Fn(Font, &mut dyn Write) -> std::fmt::Result + Send + Sync;

/// Resolves a [`Font`] to the URL of its stylesheet.
///
/// A `FontResolver` lives in the app context. A [`Font`] used as an attribute
/// value in a view reads it to render the stylesheet URL. Registering a font on
/// the router sets it up, so application code does not need to create one.
pub struct FontResolver {
    resolve_fn: Box<ResolveFontRouteFn>,
}

impl FontResolver {
    /// Creates a resolver that calls `resolve_fn`.
    #[must_use]
    pub fn new(resolve_fn: Box<ResolveFontRouteFn>) -> Self {
        Self { resolve_fn }
    }

    /// Writes the stylesheet URL of `font` to `write`.
    ///
    /// # Errors
    ///
    /// Returns any error from the resolver function.
    pub fn resolve(&self, font: Font, write: &mut dyn Write) -> std::fmt::Result {
        (self.resolve_fn)(font, write)
    }
}
