//! Paths to the framework crates, for naming their items in generated code.
//!
//! Each [`Crate`] constant resolves a path using the calling crate's
//! dependencies. A direct dependency takes precedence over a re-export
//! through `topcoat`. Dependency renames are respected.
//!
//! Interpolate a constant into `quote!` like any other path:
//!
//! ```rust
//! use quote::quote;
//! use topcoat_core_grammar::paths::topcoat_context;
//!
//! let tokens = quote! { fn helper(cx: &#topcoat_context::Cx) {} };
//! ```

use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::TokenStream;
use quote::ToTokens;

/// A framework crate, or a module within one, that generated code can refer to.
///
/// Interpolate it into `quote!` to emit the path for the calling crate's
/// dependencies.
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
/// Prefer the standalone dependency. The facade may be available only as a
/// dev-dependency, which `crate_name` cannot distinguish from a regular one.
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
/// Use the crate's external name for self references so the path also works
/// in doctests. Framework crates provide a matching `extern crate self` alias.
///
/// Resolve on each call because a proc-macro process can serve several
/// crates. The answer depends on the current `CARGO_MANIFEST_DIR`.
fn crate_base(package: &str) -> Option<String> {
    match crate_name(package) {
        Ok(FoundCrate::Itself) => Some(format!("::{}", package.replace('-', "_"))),
        Ok(FoundCrate::Name(name)) => Some(format!("::{name}")),
        Err(_) => None,
    }
}

/// `::topcoat::asset`, or `topcoat_asset` standalone.
#[allow(non_upper_case_globals)]
pub const topcoat_asset: Crate = Crate::new("asset", "topcoat-asset", "");

/// `::topcoat::context`, or `topcoat_core::context` standalone.
#[allow(non_upper_case_globals)]
pub const topcoat_context: Crate = Crate::new("context", "topcoat-core", "context");

/// Shared core types.
#[allow(non_upper_case_globals)]
pub const topcoat_core: Crate = Crate::new("core", "topcoat-core", "");

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

/// `::topcoat::mail`, or `topcoat_mail` standalone.
#[allow(non_upper_case_globals)]
pub const topcoat_mail: Crate = Crate::new("mail", "topcoat-mail", "");

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
