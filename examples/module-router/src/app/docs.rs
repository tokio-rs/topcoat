pub mod install;

use topcoat::{
    Result,
    context::Cx,
    router::{Slot, href, layout, page, path_param},
    view::{View, view},
};

path_param!(topic, segment = false);

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
        <a href=(href!(topic, Topic("routing")))>"routing topic"</a>
    })
}

// Relative parameters keep the /docs prefix without needing another module.
#[page("./topics/{topic}")]
async fn topic(cx: &Cx) -> Result<impl View> {
    let topic = path_param::<Topic>(cx);
    Ok(view! {
        <h1>
            "Topic: "
            (topic)
        </h1>
        <p>"src/app/docs.rs -> /docs/topics/{topic}"</p>
    })
}
