use topcoat::{
    Result,
    router::page,
    view::{View, view},
};

#[page]
pub async fn page() -> Result<impl View> {
    Ok(view! {
        signal show = false;

        <button @click=$(|_e| show.toggle())>
            "click to "
            $(if show.get() { "hide" } else { "reveal" })
        </button>

        // A bind attribute keeps `hidden` in sync with the signal.
        <div :hidden=$(!show.get())>"hello world!"</div>
    })
}
