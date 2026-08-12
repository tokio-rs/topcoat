mod pricing;

use topcoat::{
    Result,
    router::layout,
    view::{View, view},
};

// Underscore modules such as `_marketing` are groups: they can add a layout without adding a URL
// segment.
#[layout]
async fn marketing_layout(slot: View) -> Result {
    view! {
        <section>
            <p>"marketing group layout"</p>
            (slot?)
        </section>
    }
}
