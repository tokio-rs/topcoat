#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]

extern crate self as topcoat;

#[cfg(feature = "router")]
pub mod dev;

#[cfg(feature = "serve")]
mod serve;

pub use topcoat_core::error::Error;

#[cfg(feature = "view")]
pub type Result<T = view::View, E = topcoat_core::error::Error> = topcoat_core::error::Result<T, E>;
#[cfg(not(feature = "view"))]
pub type Result<T, E = topcoat_core::error::Error> = topcoat_core::error::Result<T, E>;

#[cfg(feature = "alpine-ajax")]
pub mod alpine_ajax;

#[cfg(feature = "asset")]
pub mod asset;

#[cfg(feature = "cookie")]
pub mod cookie;

pub mod context;

#[cfg(feature = "datastar")]
pub mod datastar;

#[cfg(feature = "font")]
pub mod font;

#[cfg(feature = "htmx")]
pub mod htmx;

#[cfg(feature = "icon")]
pub mod icon;

#[cfg(feature = "mail")]
pub mod mail;

#[cfg(feature = "shell-view")]
pub mod shell_view;

#[cfg(feature = "router")]
pub mod router;

#[cfg(feature = "view")]
pub mod view;

#[cfg(all(feature = "serve", not(feature = "vercel")))]
pub use serve::start;
#[cfg(feature = "serve")]
pub use serve::{serve, serve_until};

/// Starts a Topcoat router using the current runtime.
///
/// Applications with the `vercel` feature use the Vercel runtime when
/// deployed and the normal Topcoat server locally.
///
/// # Errors
///
/// Returns an error if the selected runtime cannot start.
#[cfg(feature = "vercel")]
pub async fn start(router: router::Router) -> Result<()> {
    if std::env::var_os("VERCEL_IPC_PATH").is_some()
        || std::env::var_os("VERCEL_DEV_PORT").is_some()
    {
        topcoat_vercel::run(router).await.map_err(Into::into)
    } else {
        serve::start(router).await.map_err(Into::into)
    }
}

#[cfg(feature = "runtime")]
pub mod runtime;

#[cfg(feature = "session")]
pub mod session;

#[cfg(feature = "tailwind")]
pub mod tailwind;

#[cfg(feature = "vercel")]
#[doc = include_str!("../docs/vercel.md")]
pub mod vercel {
    pub use topcoat_vercel::{Error, run};
}

#[doc(hidden)]
pub mod internal;
