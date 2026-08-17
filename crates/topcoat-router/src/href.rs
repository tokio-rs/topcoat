use topcoat_core::context::Cx;

use crate::Path;

/// A value resolvable to the URL path it is served at, like the marker a
/// `#[page]` or `#[route]` declaration generates.
///
/// A handler marker resolves through the router that dispatched the current
/// request to the path its route was registered under: group segments are
/// stripped and parameters keep their `{name}` form. A plain [`Path`] is its
/// own target.
pub trait HrefTarget {
    /// Returns the URL path this value is served at.
    ///
    /// # Panics
    ///
    /// Implementations that resolve through a router panic if the context
    /// carries none, or if the target is not registered on it.
    fn path<'cx>(&self, cx: &'cx Cx) -> &'cx Path;
}

impl HrefTarget for &'static Path {
    fn path<'cx>(&self, _cx: &'cx Cx) -> &'cx Path {
        self
    }
}
