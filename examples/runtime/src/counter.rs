use topcoat::{Result, router::page, view::view};

#[page]
async fn counter() -> Result {
    view! {
        signal count = 0.0;

        <button @click=$(|_e| count.increment())>"increment"</button>
        <button @click=$(|_e| count.decrement())>"decrement"</button>

        <br>
        <br>

        $(count.get())
    }
}
