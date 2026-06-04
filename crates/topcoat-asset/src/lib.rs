//! Declare assets in source, embed references in the compiled binary, and
//! bundle the actual files out at build time.
//!
//! The [`asset!`] macro declares an asset by path and returns a compact
//! [`Asset`] identifier to use at runtime. Each invocation also embeds the
//! metadata needed to locate the file (path, crate, source location,
//! options) into the binary itself. After building, a [`Bundler`] scans
//! the binary, copies or downloads every referenced file into an output
//! directory with content-hashed names, and writes a [`Manifest`]. At
//! runtime, an [`AssetBundle`] loads that manifest and resolves [`Asset`]
//! IDs back to the bundled files.
//!
//! # Features
//!
//! - `tower` (default) — [`ServeAssetBundle`] for serving a bundle over
//!   HTTP via `tower-http`.
//! - `bundler` — the [`Bundler`] type (pulls in `tokio` and `reqwest`).
//! - `view` — integrates [`Asset`] with `topcoat-view`'s `ViewPart` system
//!   so an `Asset` can be rendered directly into a view.
//!
//! # Example
//!
//! ```ignore
//! use topcoat_asset::{asset, Asset, AssetBundle, Bundler};
//!
//! const LOGO: Asset = asset!("assets/logo.png");
//!
//! // Build step: scan the binary and emit a bundle directory.
//! Bundler::new("target/asset-cache")
//!     .bundle(&binary_bytes, "dist/assets")
//!     .await?;
//!
//! // Runtime: resolve IDs back to on-disk files.
//! let bundle = AssetBundle::load_dir("dist/assets")?;
//! let path = bundle.get(LOGO).unwrap().path();
//! ```

mod asset;
mod bundle;
mod cursor;
mod error;
mod hash;
mod manifest;
mod options;
mod source;

pub use asset::*;
pub use bundle::*;
pub use error::*;
pub use manifest::*;
pub use options::*;
pub use source::*;

#[cfg(feature = "bundler")]
mod bundler;

#[cfg(feature = "bundler")]
pub use bundler::*;

#[cfg(feature = "tower")]
mod tower;

#[cfg(feature = "tower")]
pub use tower::*;

#[cfg(feature = "view")]
mod view;

#[cfg(feature = "view")]
pub use view::*;
