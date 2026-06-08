pub use topcoat_runtime::runtime::*;
pub use topcoat_runtime_macro::expr;

use topcoat::view::{component, view};

#[component]
pub async fn script() -> topcoat::Result {
    view! {
        <script type="module" src=(topcoat::runtime::SCRIPT)></script>
    }
}
