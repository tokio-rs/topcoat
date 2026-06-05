extern crate self as topcoat;

#[cfg(feature = "router")]
pub mod dev;

#[cfg(feature = "router")]
mod serve;

pub use topcoat_core::error::Error;

#[cfg(feature = "view")]
pub type Result<T = view::View, E = topcoat_core::error::Error> = topcoat_core::error::Result<T, E>;
#[cfg(not(feature = "view"))]
pub type Result<T, E = topcoat_core::error::Error> = topcoat_core::error::Result<T, E>;

#[cfg(feature = "asset")]
pub mod asset {
    pub use topcoat_asset::*;
}

pub mod context {
    pub use topcoat_core::context::*;
    pub use topcoat_macro::memoize;
}

#[cfg(feature = "router")]
pub mod router {
    pub use topcoat_macro::{layout, page, path_param, query_params, route, segment};
    pub use topcoat_router::*;
}

#[cfg(feature = "view")]
pub mod view {
    pub use topcoat_macro::{attributes, component, shard, view};
    pub use topcoat_view::runtime::*;
}

#[cfg(feature = "router")]
pub use serve::{serve, start};

#[cfg(feature = "runtime")]
pub mod runtime {
    pub use topcoat_macro::expr;
    pub use topcoat_runtime::runtime::*;
}

#[cfg(feature = "tailwind")]
pub mod tailwind {
    pub use topcoat_tailwind::*;
}

#[doc(hidden)]
pub mod internal {
    #[cfg(feature = "discover")]
    pub use inventory;
    pub use serde;
    pub use serde_urlencoded;

    pub use topcoat_core::internal::*;
}
