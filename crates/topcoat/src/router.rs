#![doc = include_str!("../docs/router.md")]

pub use topcoat_router::runtime::*;
pub use topcoat_router_macro::*;

#[cfg(all(feature = "discover", feature = "runtime"))]
use topcoat_runtime::runtime::RouterBuilderProcedureExt;

#[cfg(feature = "discover")]
pub trait RouterBuilderDiscoverExt {
    #[must_use]
    fn discover(self) -> Self;
}

#[cfg(feature = "discover")]
impl RouterBuilderDiscoverExt for RouterBuilder {
    fn discover(mut self) -> Self {
        self = self.discover_routes();
        self = self.discover_pages();
        self = self.discover_layouts();
        self = self.discover_layers();
        #[cfg(feature = "runtime")]
        {
            self = self.discover_procedures();
        }
        self
    }
}
