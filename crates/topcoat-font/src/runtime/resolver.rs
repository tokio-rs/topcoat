use std::fmt::Write;

use crate::runtime::Font;

/// Function that formats the URL at which a [`Font`]'s CSS is hosted by the
/// router into a `dyn Write`.
pub type ResolveFontRouteFn = dyn Fn(&Font, &mut dyn Write) -> std::fmt::Result + Send + Sync;

/// Function registered with the app context that resolves a [`Font`] to its
/// hosted stylesheet URL.
///
/// Registered by [`RouterBuilderFontExt`](crate::runtime::RouterBuilderFontExt)
/// when a font is added to the router, and read when a [`Font`] is used as an
/// attribute value in the `view!` macro.
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
    pub fn resolve(&self, font: &Font, write: &mut dyn Write) -> std::fmt::Result {
        (self.resolve_fn)(font, write)
    }
}
