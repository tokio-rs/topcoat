#![cfg_attr(docsrs, feature(doc_cfg))]
//! The default Topcoat UI component registry.
//!
//! Enable Topcoat's `ui` feature to make this registry available to `topcoat ui`.
//! Its command-line name is `topcoat`.
//!
//! The manifest describes the themes and components to copy into projects.
//! Component sources also compile as test modules so changes can be checked
//! before installation.

#[cfg(all(test, feature = "stage-icons"))]
pub mod components;

/// The registry manifest and the theme and component sources it names, as
/// `(path, contents)` pairs. Paths are relative to the registry directory and
/// use `/` as the separator.
#[cfg(feature = "embedded")]
pub static FILES: &[(&str, &str)] = include!(concat!(env!("OUT_DIR"), "/embedded.rs"));
