//! Paths to the framework crates, for naming their items in generated code.
//!
//! A macro like `view!` expands to code that refers to framework types -- for
//! instance `topcoat::view::Component`. It cannot hardcode those paths, because
//! the same macro is used from crates that reach the framework in different
//! ways:
//!
//! - Application crates depend on the `topcoat` facade and name the type
//!   `::topcoat::view::Component`.
//! - Component libraries depend on the individual crates (`topcoat-view`, and so on) directly, and
//!   name it `::topcoat_view::Component` instead. Forcing them onto the facade is undesirable, and
//!   a crate that the facade itself re-exports could not depend on the facade without a cycle.
//!
//! Each framework crate is therefore represented by a [`Crate`] constant that
//! resolves to the right path for whoever is compiling the call site: straight
//! to the standalone crate when the caller depends on it directly, and through
//! the facade otherwise.
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

    /// The resolved crate path as a string, for contexts that need a string
    /// literal rather than tokens -- such as `#[serde(crate = "...")]`.
    #[must_use]
    pub fn path_string(&self) -> String {
        resolve(self)
    }
}

impl ToTokens for Crate {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let path = resolve(self);
        let path: syn::Path = syn::parse_str(&path).expect("resolved crate path is valid");
        path.to_tokens(tokens);
    }
}

/// Resolves `krate` to a path for the crate currently being compiled.
///
/// The standalone crate is preferred whenever it is a direct dependency: a
/// component library depends on the individual crates (`topcoat-view`, and so
/// on) and names them directly. This holds even when the library *also* pulls
/// the facade in as a dev-dependency for its tests -- keying off the standalone
/// crate keeps the library's own code resolving to the crates its `lib` target
/// actually links, which a facade-first check would get wrong (`crate_name`
/// cannot tell a dev-dependency from a real one). Only a caller that depends on
/// the facade alone -- an application crate -- falls through to the facade path.
fn resolve(krate: &Crate) -> String {
    if let Some(base) = crate_base(krate.package) {
        return join(&base, krate.module);
    }

    if let Some(base) = crate_base("topcoat") {
        return join(&base, krate.facade);
    }

    // Neither the crate nor the facade is a declared dependency (as in a grammar
    // crate's own unit tests). Fall back to the bare underscored crate name.
    join(
        &format!("::{}", krate.package.replace('-', "_")),
        krate.module,
    )
}

/// Joins a resolved crate base with a submodule path, e.g. `("::topcoat",
/// "view")` becomes `"::topcoat::view"`. An empty submodule leaves the base
/// untouched, as for the facade root or a crate referred to at its top level.
fn join(base: &str, submodule: &str) -> String {
    if submodule.is_empty() {
        base.to_owned()
    } else {
        format!("{base}::{submodule}")
    }
}

/// The base path to `package` when it is a direct dependency of the crate being
/// compiled: `::renamed` under whatever name the dependent gives it, or the
/// crate's own extern name when it *is* that crate. `None` when it is not a
/// dependency.
///
/// The self-referential case names the crate `::topcoat_view` rather than
/// `crate`, because `crate_name` reports `Itself` even inside a doctest -- which
/// is compiled as a *separate* crate that links the real one as an extern, where
/// `crate` would point at the doctest binary. Each framework crate carries an
/// `extern crate self as topcoat_...;` alias so that this same extern name also
/// resolves within its own non-doctest builds. See the [module docs](self).
///
/// One `rustc` process compiles one crate, so the answer is fixed for the whole
/// compilation; it is resolved once per package and cached.
fn crate_base(package: &'static str) -> Option<String> {
    static CACHE: OnceLock<Mutex<HashMap<&'static str, Option<String>>>> = OnceLock::new();
    CACHE
        .get_or_init(Mutex::default)
        .lock()
        .expect("crate path cache is not poisoned")
        .entry(package)
        .or_insert_with(|| match crate_name(package) {
            Ok(FoundCrate::Itself) => Some(format!("::{}", package.replace('-', "_"))),
            Ok(FoundCrate::Name(name)) => Some(format!("::{name}")),
            Err(_) => None,
        })
        .clone()
}

/// `::topcoat::asset`, or `topcoat_asset` standalone.
#[allow(non_upper_case_globals)]
pub const topcoat_asset: Crate = Crate::new("asset", "topcoat-asset", "");

/// `::topcoat::context`, or `topcoat_core::context` standalone.
#[allow(non_upper_case_globals)]
pub const topcoat_context: Crate = Crate::new("context", "topcoat-core", "context");

/// The `memoize` macro: `::topcoat::context`, or `topcoat_core_macro` standalone.
#[allow(non_upper_case_globals)]
pub const topcoat_context_macro: Crate = Crate::new("context", "topcoat-core-macro", "");

/// `::topcoat` (the facade root), or `topcoat_core::error` standalone: the
/// `Error` and `Result` types, which the facade re-exports at its root.
#[allow(non_upper_case_globals)]
pub const topcoat_error: Crate = Crate::new("", "topcoat-core", "error");

/// `::topcoat::internal`, or `topcoat_core::internal` standalone.
#[allow(non_upper_case_globals)]
pub const topcoat_internal: Crate = Crate::new("internal", "topcoat-core", "internal");

/// `::topcoat::internal::inventory`, or the standalone `inventory` crate that
/// the facade re-exports there.
#[allow(non_upper_case_globals)]
pub const topcoat_inventory: Crate = Crate::new("internal::inventory", "inventory", "");

/// `::topcoat::internal::serde`, or the standalone `serde` crate that the facade
/// re-exports there.
#[allow(non_upper_case_globals)]
pub const topcoat_serde: Crate = Crate::new("internal::serde", "serde", "");

/// `::topcoat::font`, or `topcoat_font` standalone.
#[allow(non_upper_case_globals)]
pub const topcoat_font: Crate = Crate::new("font", "topcoat-font", "");

/// The `font!` (and sibling) macros: `::topcoat::font`, or `topcoat_font_macro`
/// standalone.
#[allow(non_upper_case_globals)]
pub const topcoat_font_macro: Crate = Crate::new("font", "topcoat-font-macro", "");

/// `::topcoat::font::fontsource`, or `topcoat_font::fontsource` standalone: the
/// Fontsource catalog types, behind the `fontsource` feature.
#[allow(non_upper_case_globals)]
pub const topcoat_font_fontsource: Crate =
    Crate::new("font::fontsource", "topcoat-font", "fontsource");

/// The `fontsource_font_face!` (and sibling) macros:
/// `::topcoat::font::fontsource`, or `topcoat_font_macro` standalone.
#[allow(non_upper_case_globals)]
pub const topcoat_font_fontsource_macro: Crate =
    Crate::new("font::fontsource", "topcoat-font-macro", "");

/// `::topcoat::icon`, or `topcoat_icon` standalone.
#[allow(non_upper_case_globals)]
pub const topcoat_icon: Crate = Crate::new("icon", "topcoat-icon", "");

/// `::topcoat::router`, or `topcoat_router` standalone.
#[allow(non_upper_case_globals)]
pub const topcoat_router: Crate = Crate::new("router", "topcoat-router", "");

/// The `segment!` (and sibling) macros: `::topcoat::router`, or
/// `topcoat_router_macro` standalone.
#[allow(non_upper_case_globals)]
pub const topcoat_router_macro: Crate = Crate::new("router", "topcoat-router-macro", "");

/// `::topcoat::runtime`, or `topcoat_runtime` standalone.
#[allow(non_upper_case_globals)]
pub const topcoat_runtime: Crate = Crate::new("runtime", "topcoat-runtime", "");

/// The `expr!` (and sibling) macros: `::topcoat::runtime`, or
/// `topcoat_runtime_macro` standalone.
#[allow(non_upper_case_globals)]
pub const topcoat_runtime_macro: Crate = Crate::new("runtime", "topcoat-runtime-macro", "");

/// `::topcoat::view`, or `topcoat_view` standalone.
#[allow(non_upper_case_globals)]
pub const topcoat_view: Crate = Crate::new("view", "topcoat-view", "");

/// The `view!`, `component`, and `Props` (and sibling) macros:
/// `::topcoat::view`, or `topcoat_view_macro` standalone.
#[allow(non_upper_case_globals)]
pub const topcoat_view_macro: Crate = Crate::new("view", "topcoat-view-macro", "");
