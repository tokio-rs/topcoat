use futures_core::Stream;
use futures_util::stream;
use topcoat::{
    Result,
    context::Cx,
    datastar::{ElementPatchMode, PatchElements, PatchSignals, Signals},
    router::{
        content::sse::{Event, Sse},
        route,
    },
    view::{ViewExt, view},
};

use crate::Counter;

#[route(POST)]
pub(crate) async fn increment(
    cx: &Cx,
    Signals(counter): Signals<Counter>,
) -> Result<Sse<impl Stream<Item = Result<Event>> + use<>>> {
    let count = counter.count + 1;

    let entry = view! {
        cx =>
        <li>
            "Counted to "
            (count)
        </li>
    }
    .single()
    .await?;

    // One event updates the counter signal, the other appends the log entry.
    let events = stream::iter([
        PatchSignals::json(&Counter { count }).map(Into::into),
        Ok(PatchElements::new(entry.render(cx))
            .selector("#log")
            .mode(ElementPatchMode::Append)
            .into()),
    ]);

    Ok(Sse::new(events))
}
