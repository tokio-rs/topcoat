pub mod pricing;

use topcoat::{
    Result,
    router::{Slot, layout},
    view::{View, view},
};

// A module whose name starts with `_`, such as `_marketing`, is a group. It
// can add a layout without adding a URL segment.
#[layout]
async fn marketing_layout(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        <section>
            <p>"marketing group layout"</p>
            (slot)
        </section>
    })
}
