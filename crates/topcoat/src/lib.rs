extern crate self as topcoat;

#[cfg(feature = "router")]
pub mod dev;

#[cfg(feature = "router")]
mod serve;

#[cfg(feature = "asset")]
pub mod asset {
    pub use topcoat_asset::*;
}

pub mod context {
    pub use topcoat_macro::memoize;

    pub use topcoat_core::context::*;
}

#[cfg(feature = "router")]
pub mod router {
    pub use topcoat_macro::{layout, page, path_param, query_params, route, segment};

    pub use topcoat_router::*;
}

#[cfg(feature = "view")]
pub mod view {
    pub use topcoat_macro::{component, island, view};

    pub use topcoat_view::runtime::*;
}

#[cfg(feature = "router")]
pub use serve::serve;

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

    pub trait ResultExt {
        type T;
        type E;
    }

    impl<T, E> ResultExt for Result<T, E> {
        type T = T;
        type E = E;
    }
}
