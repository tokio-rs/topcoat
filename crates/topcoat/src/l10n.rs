pub use topcoat_l10n::*;

#[cfg(feature = "l10n-fluent")]
pub mod fluent {
    #[doc(hidden)]
    pub use topcoat_l10n_macro::__t;
    pub use topcoat_l10n_macro::messages;
}
