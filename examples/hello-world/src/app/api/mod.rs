use axum::http::StatusCode;
use topcoat::router::{IntoResponse, Response, Result, route};

#[route(GET)]
async fn kek() -> Result<Response> {
    Ok((StatusCode::OK).into_response())
}
