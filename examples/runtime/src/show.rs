use topcoat::{Result, router::page, view::view};

#[page]
async fn show() -> Result {
    view! {
        // Store the visibility state in the browser.
        signal show = false;

        // Toggle the signal and update the button label.
        <button @click=$(|_e| show.toggle())>
            "click to "
            $(if show.get() { "hide" } else { "reveal" })
        </button>

        // Keep the hidden attribute synchronized with the signal.
        <div :hidden=$(!show.get())>"hello world!"</div>
    }
}
