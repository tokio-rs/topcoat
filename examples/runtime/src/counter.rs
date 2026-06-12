use topcoat::{Result, router::page, runtime::Signal, view::view};

#[page]
async fn counter() -> Result {
    view! {
        client let count = Signal::new(0.0);

        <button @click=$(|_e| count.set(count.get() + 1.0))>"increment"</button>
        <button @click=$(|_e| count.set(count.get() - 1.0))>"decrement"</button>

        <br>
        <br>

        $(count.get())
    }
}
