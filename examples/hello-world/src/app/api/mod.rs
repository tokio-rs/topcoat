use axum::http::StatusCode;
use topcoat::{
    Result,
    router::{IntoResponse, Response, route},
};

#[route(GET)]
async fn kek() -> Result<Response> {
    (StatusCode::OK).into_response()
}
