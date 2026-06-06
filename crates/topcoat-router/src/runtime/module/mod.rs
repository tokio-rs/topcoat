mod layout;
mod page;
mod route;
mod router;
mod segment;

pub use layout::*;
pub use page::*;
pub use route::*;
pub use router::*;
pub use segment::*;

#[cfg(feature = "discover")]
#[macro_export]
macro_rules! module_router {
    () => {
        ::topcoat::router::Router::from(
            ::topcoat::router::ModuleRouter::new(module_path!()).discover(),
        )
    };
}
pub use module_router;
