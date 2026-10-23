use topcoat::{
    Result,
    router::page,
    view::{View, view},
};

use crate::app::t;

#[page]
pub async fn page() -> Result<impl View> {
    Ok(view! {
        <h1>(t!(title))</h1>
        <p>(t!(intro))</p>
        <p>(t!(greeting, name = "Ada"))</p>
        // The wording depends on the count, and the number is formatted for the locale.
        <ul>
            for count in [1, 5, 1234] {
                <li>(t!(items, count))</li>
            }
        </ul>
        // A translation can also be an attribute value.
        <input type="search" aria-label=(t!(search)) placeholder=(t!(search.placeholder))>
    })
}
