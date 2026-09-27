use std::time::Duration;

use futures_core::Stream;
use futures_util::stream;
use topcoat::{
    Result,
    router::{
        content::sse::{Event, Sse},
        route,
    },
};

use crate::Progress;

#[route(GET)]
pub(crate) async fn job() -> Result<Sse<impl Stream<Item = Result<Event>> + use<>>> {
    // Unlike the ticks, this stream ends: the browser closes the connection
    // once it receives the `done` event.
    let events = stream::unfold(0, |percent| async move {
        if percent > 100 {
            return None;
        }

        tokio::time::sleep(Duration::from_millis(400)).await;

        let event = if percent == 100 {
            Ok(Event::new().event("done").data("finished"))
        } else {
            Event::new()
                .event("progress")
                .json_data(&Progress { percent })
        };

        Some((event, percent + 10))
    });

    Ok(Sse::new(events))
}
