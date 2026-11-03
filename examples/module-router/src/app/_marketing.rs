pub mod pricing;

use topcoat::{
    Result,
    context::Cx,
    router::{Slot, layout},
    view::{View, view},
};

// Underscore modules such as `_marketing` are groups: they can add a layout without adding a URL
// segment.
#[layout]
async fn marketing_layout(cx: &Cx, slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        <section>
            <p>"marketing group layout"</p>
            (slot)
        </section>
    })
}
