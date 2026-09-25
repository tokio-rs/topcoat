use std::fmt::Write;

use crate::Font;

/// A callback that writes a [`Font`]'s stylesheet URL.
pub type ResolveFontRouteFn = dyn Fn(Font, &mut dyn Write) -> std::fmt::Result + Send + Sync;

/// Resolves fonts to their hosted stylesheet URLs.
///
/// Register in app context to control the URLs rendered for [`Font`] values.
pub struct FontResolver {
    resolve_fn: Box<ResolveFontRouteFn>,
}

impl FontResolver {
    /// Builds a resolver from a callback.
    #[must_use]
    pub fn new(resolve_fn: Box<ResolveFontRouteFn>) -> Self {
        Self { resolve_fn }
    }

    /// Invokes the underlying callback.
    ///
    /// # Errors
    ///
    /// Propagates errors of the registered [`ResolveFontRouteFn`].
    pub fn resolve(&self, font: Font, write: &mut dyn Write) -> std::fmt::Result {
        (self.resolve_fn)(font, write)
    }
}
