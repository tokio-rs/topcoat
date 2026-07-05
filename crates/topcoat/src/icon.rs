pub use topcoat_icon::*;

#[cfg(feature = "icon-iconify")]
pub mod iconify {
    pub use topcoat_icon_iconify::*;
}

#[cfg(feature = "view")]
#[topcoat::view::component]
pub async fn icon(data: IconData) {
    topcoat::view::view! { <svg></svg> }
}
