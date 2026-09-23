#![doc = include_str!("../docs/icon.md")]

pub use topcoat_icon::*;

/// Icon sets from [Iconify](https://icon-sets.iconify.design/).
///
/// See the [Iconify section](crate::icon#iconify) of the icon guide.
#[cfg(feature = "icon-iconify")]
pub mod iconify {
    pub use topcoat_icon::iconify::*;
    pub use topcoat_icon_macro::{iconify_icon, include};
}
