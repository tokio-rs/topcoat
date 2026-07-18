#![doc = include_str!("../docs/runtime.md")]

pub use topcoat_runtime::*;
pub use topcoat_runtime_macro::*;

#[cfg(feature = "view")]
#[cfg_attr(docsrs, doc(cfg(feature = "view")))]
#[topcoat::view::component]
pub async fn script() -> topcoat::Result {
    topcoat::view::view! {
        <script type="module" src=(topcoat::runtime::SCRIPT)></script>
    }
}
