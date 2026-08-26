pub mod install;

use topcoat::{
    Result,
    router::{Slot, layout, page},
    view::{View, view},
};

// A layout in app::docs wraps /docs and child routes such as /docs/install.
#[layout]
async fn docs_layout(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        <section>
            <p>"docs layout"</p>
            (slot)
        </section>
    })
}

// A page in app::docs renders at /docs.
#[page]
pub async fn page() -> Result<impl View> {
    Ok(view! {
        <h1>"docs"</h1>
        <p>"src/app/docs.rs -> /docs"</p>
    })
}
