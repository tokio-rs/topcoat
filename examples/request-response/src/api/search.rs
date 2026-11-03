use topcoat::{
    Result,
    context::Cx,
    router::{
        content::{Form, Json},
        route,
    },
};

use crate::{Search, SearchResult};

// For GET and HEAD requests, Form<T> reads URL-encoded values from the query string.
#[route(GET)]
pub(crate) async fn search(_cx: &Cx, Form(input): Form<Search>) -> Result<Json<SearchResult>> {
    Ok(Json(SearchResult {
        query: input.q,
        limit: input.limit.unwrap_or(10),
    }))
}
