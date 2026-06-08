use topcoat::{Result, router::page, view::view};

#[page]
async fn show() -> Result {
    view! {
        signal show = false;

        <button @click=$(|_e| show.set(!show.get()))>
            "click to "
            $(if show.get() { "hide" } else { "reveal" })
        </button>

        <div :style=$(if show.get() { "display: block" } else { "display: none" })>
            "hello world!"
        </div>
    }
}
