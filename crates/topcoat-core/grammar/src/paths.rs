//! Paths to the framework crates, for naming their items in generated code.
//!
//! A macro like `view!` expands to code that refers to framework types -- for
//! instance `topcoat::view::Component`. It cannot hardcode those paths, because
//! the same macro is used from crates that reach the framework in different
//! ways:
//!
//! - Application crates depend on the `topcoat` facade and name the type
//!   `::topcoat::view::Component`.
//! - Component libraries depend on the individual crates (`topcoat-view`, and
//!   so on) directly, and name it `::topcoat_view::Component` instead. Forcing
//!   them onto the facade is undesirable, and a crate that the facade itself
//!   re-exports could not depend on the facade without a cycle.
//!
//! Each framework crate is therefore represented by a [`Crate`] constant that
//! resolves to the right path for whoever is compiling the call site: through
//! the facade when the caller depends on it, and straight to the standalone
//! crate otherwise.
//!
//! Interpolate a constant into `quote!` like any other path:
//!
//! ```ignore
//! use topcoat_core_grammar::paths::{topcoat_context, topcoat_view};
//!
//! quote! {
//!     impl #topcoat_view::Component for #ident {
//!         async fn render(self, cx: &#topcoat_context::Cx) -> #ret { ... }
//!     }
//! }
//! ```

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::TokenStream;
use quote::ToTokens;

/// A framework crate, or a module within one, that generated code can refer to.
///
/// Interpolate a `Crate` into a `quote!` invocation to emit its path. See the
/// [module docs](self) for why the path is resolved rather than hardcoded: it
/// becomes the facade path when the call site depends on `topcoat`, and the
/// standalone-crate path otherwise.
pub struct Crate {
    /// Path within the `topcoat` facade, e.g. `"view"` for `::topcoat::view`.
    /// Empty for the facade root, `::topcoat` itself.
    facade: &'static str,
    /// Cargo package name of the standalone crate, e.g. `"topcoat-view"`.
    package: &'static str,
    /// Module path within the standalone crate, used when the facade flattens a
    /// submodule that lives deeper in its own crate (e.g. `::topcoat::context`
    /// is `topcoat_core::context`). Empty for the crate root.
    module: &'static str,
}

impl Crate {
    const fn new(facade: &'static str, package: &'static str, module: &'static str) -> Self {
        Self {
            facade,
            package,
            module,
        }
    }
}

impl ToTokens for Crate {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let path = resolve(self);
        let path: syn::Path = syn::parse_str(&path).expect("resolved crate path is valid");
        path.to_tokens(tokens);
    }
}

/// Resolves `krate` to a path for the crate currently being compiled: a
/// facade-relative path when the call site depends on `topcoat`, and a path into
/// the standalone crate otherwise.
fn resolve(krate: &Crate) -> String {
    if let Some(base) = facade() {
        return if krate.facade.is_empty() {
            base.to_owned()
        } else {
            format!("{base}::{}", krate.facade)
        };
    }

    let base = package(krate.package);
    if krate.module.is_empty() {
        base
    } else {
        format!("{base}::{}", krate.module)
    }
}

/// The base path to the `topcoat` facade: `::topcoat`, or `crate` when the
/// facade itself is being compiled. `None` when the crate being compiled does
/// not depend on the facade.
///
/// One `rustc` process compiles one crate, so the answer is fixed for the whole
/// compilation; it is resolved once and cached.
fn facade() -> Option<&'static str> {
    static CACHE: OnceLock<Option<String>> = OnceLock::new();
    CACHE
        .get_or_init(|| match crate_name("topcoat") {
            Ok(FoundCrate::Itself) => Some("crate".to_owned()),
            Ok(FoundCrate::Name(name)) => Some(format!("::{name}")),
            Err(_) => None,
        })
        .as_deref()
}

/// The base path to a standalone crate: `crate` when it's the one being
/// compiled, `::renamed` when the caller renamed it, or `::package_name` as a
/// fallback. Cached per package name.
fn package(package: &'static str) -> String {
    static CACHE: OnceLock<Mutex<HashMap<&'static str, String>>> = OnceLock::new();
    CACHE
        .get_or_init(Mutex::default)
        .lock()
        .expect("crate path cache is not poisoned")
        .entry(package)
        .or_insert_with(|| match crate_name(package) {
            Ok(FoundCrate::Itself) => "crate".to_owned(),
            Ok(FoundCrate::Name(name)) => format!("::{name}"),
            Err(_) => format!("::{}", package.replace('-', "_")),
        })
        .clone()
}

/// `::topcoat` (the facade root), or `topcoat_core::error` standalone: the
/// `Error` and `Result` types, which the facade re-exports at its root.
#[allow(non_upper_case_globals)]
pub const topcoat_error: Crate = Crate::new("", "topcoat-core", "error");

/// `::topcoat::view`, or `topcoat_view` standalone.
#[allow(non_upper_case_globals)]
pub const topcoat_view: Crate = Crate::new("view", "topcoat-view", "");

/// `::topcoat::context`, or `topcoat_core::context` standalone.
#[allow(non_upper_case_globals)]
pub const topcoat_context: Crate = Crate::new("context", "topcoat-core", "context");

/// `::topcoat::runtime`, or `topcoat_runtime` standalone.
#[allow(non_upper_case_globals)]
pub const topcoat_runtime: Crate = Crate::new("runtime", "topcoat-runtime", "");

/// `::topcoat::router`, or `topcoat_router` standalone.
#[allow(non_upper_case_globals)]
pub const topcoat_router: Crate = Crate::new("router", "topcoat-router", "");

/// `::topcoat::internal`, or `topcoat_core::internal` standalone.
#[allow(non_upper_case_globals)]
pub const topcoat_internal: Crate = Crate::new("internal", "topcoat-core", "internal");

/// `::topcoat::internal::inventory`, or the standalone `inventory` crate that
/// the facade re-exports there.
#[allow(non_upper_case_globals)]
pub const topcoat_inventory: Crate = Crate::new("internal::inventory", "inventory", "");
