#![doc = include_str!("../docs/icon.md")]

pub use topcoat_icon::*;

#[cfg(feature = "icon-iconify")]
pub mod iconify {
    pub use topcoat_icon_iconify::*;
    pub use topcoat_icon_iconify_macro::*;
}
