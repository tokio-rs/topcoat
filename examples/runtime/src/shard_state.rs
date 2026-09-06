use topcoat::{
    Result,
    context::Cx,
    router::page,
    runtime::{Event, shard, signal},
    view::{View, view},
};

#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    let label = signal(cx, || String::from("clicks"));

    Ok(view! {
        <p>"Typing re-renders the card on the server; its own counter keeps counting."</p>

        <input :value=$(label.get()) @input=$(|e: Event| label.set(e.target.value))>

        <br>
        <br>

        card(label: $(label.get()))
    })
}

#[shard]
async fn card(cx: &Cx, label: String) -> Result<impl View> {
    // Created inside the shard, so it belongs to the shard's content. Its
    // current value travels with every re-render request and the signal
    // resumes from it instead of starting over at zero.
    let count = signal(cx, || 0.0);

    Ok(view! {
        <fieldset>
            <legend>(label)</legend>

            <button @click=$(|_e| count.increment())>"+1"</button>
            " "
            $(count.get())
        </fieldset>
    })
}
