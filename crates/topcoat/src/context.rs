//! [`Cx`] is Topcoat's request context. Pages, layouts, components, and routes can take it as an
//! optional parameter when they need request-scoped information.
//!
//! Add `cx: &Cx` to the function signature when needed; leave it out when the function does not
//! need request context. Topcoat passes it automatically when the parameter is present.
//!
//! ## Router request helpers
//!
//! The [`router`](crate::router) module exposes small functions for reading HTTP request data from
//! `cx`.
//!
//! ```rust
//! use topcoat::{
//!     context::Cx,
//!     router::{headers, method, uri},
//! };
//!
//! fn request_summary(cx: &Cx) -> String {
//!     let user_agent = headers(cx)
//!         .get("user-agent")
//!         .and_then(|value| value.to_str().ok())
//!         .unwrap_or("unknown");
//!
//!     format!("{} {} from {user_agent}", method(cx), uri(cx).path())
//! }
//! ```
//!
//! Available request helpers:
//!
//! - [`parts(cx)`](crate::router::parts) returns the current request's `http::request::Parts`.
//! - [`method(cx)`](crate::router::method) returns the HTTP method.
//! - [`uri(cx)`](crate::router::uri) returns the request URI.
//! - [`version(cx)`](crate::router::version) returns the HTTP version.
//! - [`headers(cx)`](crate::router::headers) returns the request headers.
//! - [`content_type(cx)`](crate::router::content_type) returns the request `Content-Type` as
//!   `Option<&str>`.
//! - [`extensions(cx)`](crate::router::extensions) returns request extensions.
//!
//! Use [`parts(cx)`](crate::router::parts) when you need several fields at once:
//!
//! ```rust
//! use topcoat::{context::Cx, router::parts};
//!
//! fn cache_key(cx: &Cx) -> String {
//!     let parts = parts(cx);
//!     format!("{}:{}", parts.method, parts.uri)
//! }
//! ```
//!
//! Use [`extensions(cx)`](crate::router::extensions) for typed request values attached by a
//! lower-level request layer or service integration:
//!
//! ```rust
//! use topcoat::{context::Cx, router::extensions};
//!
//! struct RequestId(String);
//!
//! fn request_id(cx: &Cx) -> Option<&str> {
//!     extensions(cx).get::<RequestId>().map(|id| id.0.as_str())
//! }
//! ```
//!
//! ## Path and query helpers
//!
//! Path and query parameter macros generate `of(cx)` helpers. They parse lazily and memoize the
//! parsed value for the request.
//!
//! ```rust,ignore
//! use topcoat::{
//!     Result,
//!     context::Cx,
//!     router::{page, path_param, query_params},
//!     view::view,
//! };
//!
//! #[path_param]
//! struct PostId(uuid::Uuid);
//!
//! #[query_params]
//! struct PostQuery {
//!     preview: Option<bool>,
//! }
//!
//! #[page("/posts/{post_id}")]
//! async fn post(cx: &Cx) -> Result {
//!     let post_id = PostId::of(cx).unwrap();
//!     let query = PostQuery::of(cx).unwrap();
//!
//!     view! {
//!         <article data-preview=(query.preview.unwrap_or(false))>
//!             "post id: " (post_id.to_string())
//!         </article>
//!     }
//! }
//! ```
//!
//! ## App and request context helpers
//!
//! This module exposes typed context accessors:
//!
//! - [`app_context::<T>(cx)`](app_context) reads values registered on the router with
//!   `.app_context(value)`.
//! - [`request_context::<T>(cx)`](request_context) reads typed values attached to the current
//!   request.
//!
//! ```rust
//! use topcoat::context::{Cx, app_context};
//! #
//! # struct Database;
//!
//! fn db(cx: &Cx) -> &Database {
//!     app_context::<Database>(cx)
//! }
//! ```
//!
//! Values are keyed by Rust type. Asking for a type that was not registered panics, so these
//! helpers are best wrapped in small application-specific functions like `db(cx)`, `config(cx)`, or
//! `current_tenant(cx)`.
//!
//! ## Request body parsing
//!
//! Handlers can receive one request body parameter in addition to `cx: &`[`Cx`]. The parameter can
//! be any type that implements [`FromRequest`](crate::router::FromRequest), including Topcoat's
//! built-in [`Json<T>`](crate::router::Json), [`Form<T>`](crate::router::Form), optional
//! `Multipart`, [`Body`](crate::router::Body), [`Bytes`](crate::router::Bytes), and `String`
//! extractors.
//!
//! ## Composing helpers
//!
//! Any helper can accept `cx: &`[`Cx`], call other helpers, and return a domain-specific result:
//!
//! ```rust
//! use topcoat::{
//!     context::Cx,
//!     router::{headers, uri},
//! };
//!
//! fn locale(cx: &Cx) -> &str {
//!     headers(cx)
//!         .get("accept-language")
//!         .and_then(|value| value.to_str().ok())
//!         .and_then(|value| value.split(',').next())
//!         .unwrap_or("en")
//! }
//!
//! fn canonical_url(cx: &Cx) -> String {
//!     format!("https://example.com{}", uri(cx).path())
//! }
//! ```
//!
//! That keeps pages, layouts, components, and routes focused on rendering or responding while
//! shared request reads stay in ordinary Rust functions.

pub use topcoat_core::runtime::context::*;
pub use topcoat_core_macro::memoize;
