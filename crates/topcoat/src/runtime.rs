pub use topcoat_runtime::*;
pub use topcoat_runtime_macro::*;

#[cfg(feature = "view")]
#[topcoat::view::component]
pub async fn script() -> topcoat::Result {
    topcoat::view::view! {
        <script type="module" src=(topcoat::runtime::SCRIPT)></script>
    }
}
