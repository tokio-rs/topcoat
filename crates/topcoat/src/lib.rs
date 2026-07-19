#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]

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
pub mod asset;

#[cfg(feature = "cookie")]
pub mod cookie;

pub mod context;

#[cfg(feature = "font")]
pub mod font;

#[cfg(feature = "htmx")]
pub mod htmx;

#[cfg(feature = "icon")]
pub mod icon;

#[cfg(feature = "router")]
pub mod router;

#[cfg(feature = "view")]
pub mod view;

#[cfg(feature = "router")]
pub use serve::{serve, start};

#[cfg(feature = "runtime")]
pub mod runtime;

#[cfg(feature = "session")]
pub mod session;

#[cfg(feature = "tailwind")]
pub mod tailwind;

#[doc(hidden)]
pub mod internal;
