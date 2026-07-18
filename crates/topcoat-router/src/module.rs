mod layer;
mod page;
mod route;
mod router;
mod segment;

pub use layer::*;
pub use page::*;
pub use route::*;
pub use router::*;
pub use segment::*;

#[doc = include_str!("../docs/module_router.md")]
#[cfg(feature = "discover")]
#[cfg_attr(docsrs, doc(cfg(feature = "discover")))]
#[macro_export]
macro_rules! module_router {
    () => {
        ::topcoat::router::RouterBuilder::from(
            ::topcoat::router::ModuleRouterBuilder::new(module_path!()).discover(),
        )
    };
}
