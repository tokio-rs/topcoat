pub use topcoat_l10n::*;

#[cfg(feature = "l10n-fluent")]
pub mod fluent {
    pub use topcoat_l10n_macro::{messages, t};
}
