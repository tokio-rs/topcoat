use topcoat::{Result, router::page, runtime::Signal, view::view};

#[page]
async fn show() -> Result {
    view! {
        client let show = Signal::new(false);

        <button @click=$(|_e| show.set(!show.get()))>
            "click to "
            $(if show.get() {
                "hide"
            } else {
                "reveal"
            })
        </button>

        <div :style=$((!show.get()).then_some("display: none"))>
            "hello world!"
        </div>
    }
}
