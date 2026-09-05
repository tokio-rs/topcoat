use topcoat::{
    Result,
    context::Cx,
    router::page,
    runtime::signal,
    view::{View, view},
};

#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    let show = signal(cx, || false);

    Ok(view! {
        <button @click=$(|_e| show.toggle())>
            "click to "
            $(if show.get() { "hide" } else { "reveal" })
        </button>

        // A bind attribute keeps `hidden` in sync with the signal.
        <div :hidden=$(!show.get())>"hello world!"</div>
    })
}
