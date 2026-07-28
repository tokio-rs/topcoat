use topcoat::{Result, router::page, view::view};

#[page]
async fn counter() -> Result {
    view! {
        // Store the counter as reactive state in the browser.
        signal count = 0.0;

        // Update the signal when a button is clicked.
        <button @click=$(|_e| count.increment())>"increment"</button>

        <button @click=$(|_e| count.decrement())>"decrement"</button>

        <br>
        <br>

        // Re-render the value whenever the signal changes.
        $(count.get())
    }
}
