use topcoat::{
    Result,
    context::Cx,
    router::page,
    runtime::signal,
    view::{View, view},
};

#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    // The signal lives in the browser; the handlers below update it.
    let count = signal(cx, || 0.0);

    Ok(view! {
        <button @click=$(|_e| count.increment())>"increment"</button>

        <button @click=$(|_e| count.decrement())>"decrement"</button>

        <br>
        <br>

        // Renders again whenever the signal changes.
        $(count.get())
    })
}
