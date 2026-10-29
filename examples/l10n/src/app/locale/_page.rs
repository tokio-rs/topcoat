use jiff::Zoned;
use topcoat::{
    Result,
    router::page,
    view::{View, view},
};

use crate::app::t;

#[page]
pub async fn page() -> Result<impl View> {
    Ok(view! {
        // Translates the message `title` into the locale of the page.
        <h1>(t!(title))</h1>
        <p>(t!(intro))</p>

        // Arguments fill in the `{ $name }` in the message.
        <p>(t!(greeting, name: "Ada"))</p>

        // Fluent picks the plural form, like "One item" or "5 items", and formats the
        // number for the locale, like 1,234 or 1.234.
        <ul>
            for count in [1, 5, 1234] {
                // Short for `count = count`.
                <li>(t!(items, count))</li>
            }
        </ul>

        // `jiff` values format as dates. `Zoned::now` is the current date and time in
        // the server's time zone.
        <p>(t!(today, date: Zoned::now()))</p>

        // `search.placeholder` is the `.placeholder` attribute of the `search` message.
        <input
            type="search"
            aria-label=(t!(search))
            placeholder=(t!(search.placeholder))
        >
    })
}
