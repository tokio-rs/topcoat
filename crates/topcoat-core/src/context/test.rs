//! Test support for constructing request contexts.

use std::any::Any;

use super::Cx;

/// Assembles a [`Cx`] from scratch for tests.
#[derive(Debug, Default)]
pub struct CxTestBuilder {
    cx: Cx,
}

impl CxTestBuilder {
    /// Creates an empty builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers `value` on the app context.
    #[must_use]
    pub fn app_context<T>(mut self, value: T) -> Self
    where
        T: Any + Send + Sync,
    {
        let _ = self.cx.app_context_mut().insert(value);
        self
    }

    /// Registers `value` on the request root context.
    #[must_use]
    pub fn request_context<T>(mut self, value: T) -> Self
    where
        T: Any + Send + Sync,
    {
        let _ = self.cx.insert(value);
        self
    }

    /// Consumes the builder, returning the assembled [`Cx`].
    #[must_use]
    pub fn build(self) -> Cx {
        self.cx
    }
}
