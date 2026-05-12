pub type Response = axum::response::Response;

pub trait IntoResponse {
    fn into_response(self) -> Response;
}

impl<T> IntoResponse for T
where
    T: axum::response::IntoResponse,
{
    fn into_response(self) -> Response {
        axum::response::IntoResponse::into_response(self)
    }
}
