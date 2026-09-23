//! Parsing and code generation for Topcoat's view macros: `view!`, `live!`,
//! `emit!`, `attributes!`, `class!`, `#[component]`, and `#[derive(Props)]`.
//!
//! Each macro body parses into a syntax tree type from this crate, and the
//! type's [`ToTokens`](quote::ToTokens) implementation generates the macro's
//! expansion. Behind the `pretty` feature, the syntax trees also implement the
//! pretty-printer that `topcoat fmt` uses to format macro bodies.
//!
//! This crate is only used by Topcoat's macro crates and the `topcoat` CLI.

#![cfg_attr(docsrs, feature(doc_cfg))]

/// Attribute lists: the body of `attributes!` and the attributes of an
/// element's opening tag.
pub mod attributes;
/// The `class!` macro.
pub mod class;
/// The `#[component]` attribute macro.
pub mod component;
/// The optional leading `cx =>` argument shared by several macros.
pub mod leading_cx;
/// The `live!` and `emit!` macros.
pub mod live;
/// The `#[derive(Props)]` macro.
pub mod props;
/// Control flow and expressions shared by view bodies and attribute lists.
pub mod template;
/// The `view!` macro.
pub mod view;
