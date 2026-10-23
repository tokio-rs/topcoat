use topcoat::{
    Result,
    router::page,
    view::{View, view},
};

use crate::app::t;

#[page]
pub async fn page() -> Result<impl View> {
    Ok(view! {
        // `t!(title)` shows the message `title` in the visitor's language.
        <h1>(t!(title))</h1>
        <p>(t!(intro))</p>

        // `name = "Ada"` fills in `{ $name }` in the message.
        <p>(t!(greeting, name = "Ada"))</p>

        // The text changes with the number, as in "One item" and "5 items". Large numbers
        // are written the local way: 1,234 in English and 1.234 in German.
        <ul>
            for count in [1, 5, 1234] {
                // `count` is short for `count = count`.
                <li>(t!(items, count))</li>
            }
        </ul>

        // A translation can also be used as an attribute. `search.placeholder` is the
        // `placeholder` part of the `search` message.
        <input
            type="search"
            aria-label=(t!(search))
            placeholder=(t!(search.placeholder))
        >
    })
}
