#![doc = include_str!("../docs/icon.md")]

pub use topcoat_icon::*;

#[cfg(feature = "icon-iconify")]
pub mod iconify {
    pub use topcoat_icon::iconify::*;
    pub use topcoat_icon_macro::{iconify_icon, include};
}
