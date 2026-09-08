use topcoat::{
    Result,
    context::Cx,
    router::page,
    runtime::signal,
    view::{View, view},
};

#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    let descending = signal(cx, || false);

    // Reading the signal on the server makes the page depend on it: when
    // the button below changes it in the browser, the page runs again on
    // the server with the current value and its content is replaced.
    let mut fruit = FRUIT;
    if descending.get() {
        fruit.reverse();
    }

    Ok(view! {
        <button @click=$(|_e| descending.toggle())>
            "sort "
            $(if descending.get() { "ascending" } else { "descending" })
        </button>

        <br>
        <br>

        for item in fruit {
            <div>(item)</div>
        }
    })
}

const FRUIT: [&str; 6] = ["apple", "banana", "cherry", "date", "fig", "grape"];
