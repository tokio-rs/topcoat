mod page;
mod route;
mod router;
mod segment;

pub use page::*;
pub use route::*;
pub use router::*;
pub use segment::*;

#[cfg(feature = "discover")]
#[macro_export]
macro_rules! module_router {
    () => {
        ::topcoat::router::RouterBuilder::from(
            ::topcoat::router::ModuleRouterBuilder::new(module_path!()).discover(),
        )
    };
}
#[cfg(feature = "discover")]
pub use module_router;
