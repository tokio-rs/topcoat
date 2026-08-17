use topcoat::{Result, router::page, view::view};

#[page]
pub async fn page() -> Result {
    view! {
        // The signal lives in the browser; the handlers below update it.
        signal count = 0.0;

        <button @click=$(|_e| count.increment())>"increment"</button>

        <button @click=$(|_e| count.decrement())>"decrement"</button>

        <br>
        <br>

        // Renders again whenever the signal changes.
        $(count.get())
    }
}
