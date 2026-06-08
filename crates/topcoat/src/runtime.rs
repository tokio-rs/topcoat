pub use topcoat_runtime::runtime::*;
pub use topcoat_runtime_macro::expr;

use topcoat::{
    Result,
    view::{component, view},
};

#[component]
pub async fn script() -> Result {
    view! {
        <script type="module" src=(topcoat::runtime::SCRIPT)></script>
    }
}
